use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::alloc::{GcObjectHeader, MemoryStrategy, TypeInfo, ruyi_alloc, ruyi_dealloc};

use super::barrier::WriteBarrier;
use super::old::OldGeneration;
use super::roots::RootSet;
use super::young::YoungGeneration;

/// Generational garbage collector.
///
/// The heap is split into a young generation (copying collector) and
/// an old generation (mark-compact).  New objects are allocated in
/// the young generation.  Objects that survive a configurable number
/// of minor collections are promoted to the old generation.
///
/// Cross-generational references from old to young are tracked with a
/// write barrier and a remembered set.
pub struct GenerationalCollector {
    young: YoungGeneration,
    old: OldGeneration,
    barrier: WriteBarrier,
    roots: RootSet,
    enabled: bool,
    young_collections: AtomicUsize,
    full_collections: AtomicUsize,
}

impl GenerationalCollector {
    pub fn new() -> Self {
        Self {
            young: YoungGeneration::new(),
            old: OldGeneration::new(),
            barrier: WriteBarrier::new(),
            roots: RootSet::new(),
            enabled: true,
            young_collections: AtomicUsize::new(0),
            full_collections: AtomicUsize::new(0),
        }
    }

    pub fn with_threshold(promotion_threshold: u8) -> Self {
        Self {
            young: YoungGeneration::with_threshold(promotion_threshold),
            old: OldGeneration::new(),
            barrier: WriteBarrier::new(),
            roots: RootSet::new(),
            enabled: true,
            young_collections: AtomicUsize::new(0),
            full_collections: AtomicUsize::new(0),
        }
    }

    /// Allocate a GC-managed object in the young generation.
    ///
    /// # Safety
    /// `type_info` must remain valid for the lifetime of the object.
    pub unsafe fn allocate(&self, size: usize, type_info: *mut TypeInfo) -> *mut u8 {
        self.young.allocate(size, type_info)
    }

    /// Register a pointer as a GC root (treated as a stack root).
    ///
    /// # Safety
    /// `ptr` must point to the payload of a valid GC object.
    pub unsafe fn add_root(&self, ptr: *mut u8) {
        self.roots.add_stack_root(ptr);
    }

    /// Unregister a previously registered root.
    ///
    /// # Safety
    /// `ptr` must have been previously passed to `add_root`.
    pub unsafe fn remove_root(&self, ptr: *mut u8) {
        self.roots.remove_stack_root(ptr);
    }

    /// Register a global root.
    ///
    /// # Safety
    /// `ptr` must point to the payload of a valid GC object.
    pub unsafe fn add_global_root(&self, ptr: *mut u8) {
        self.roots.add_global_root(ptr);
    }

    /// Unregister a global root.
    ///
    /// # Safety
    /// `ptr` must have been previously passed to `add_global_root`.
    pub unsafe fn remove_global_root(&self, ptr: *mut u8) {
        self.roots.remove_global_root(ptr);
    }

    /// Run a minor collection (young generation only).
    pub fn collect_young(&self) {
        if !self.enabled {
            return;
        }

        let young_objects = self.young.objects();
        if young_objects.is_empty() {
            return;
        }

        let roots = self.roots.all_roots();
        let remembered = self.barrier.remembered_set();

        for &header in &young_objects {
            unsafe { (*header).forwarding_ptr = std::ptr::null_mut() };
        }

        let mut worklist: Vec<*mut GcObjectHeader> = Vec::new();

        // Initialise worklist with roots (stack + remembered set).
        for &root in roots.iter().chain(remembered.iter()) {
            unsafe {
                if root.is_null() {
                    continue;
                }
                if (*root).generation() >= 2 || (*root).is_pinned() {
                    // Old-generation or pinned objects are traced in place.
                    worklist.push(root);
                } else if (*root).forwarding_ptr.is_null() {
                    let new_header = self.copy_or_promote_young(root);
                    if !new_header.is_null() {
                        (*root).forwarding_ptr = (*new_header).payload();
                        worklist.push(new_header);
                    }
                } else {
                    let new_header = GcObjectHeader::from_payload((*root).forwarding_ptr);
                    worklist.push(new_header);
                }
            }
        }

        // Iteratively trace all reachable objects.
        while let Some(header) = worklist.pop() {
            unsafe {
                let type_info = (*header).type_info;
                if type_info.is_null() {
                    continue;
                }
                if let Some(trace) = (*type_info).trace_fn {
                    let payload = (*header).payload();
                    trace(payload, &mut |field: *mut *mut u8| {
                        if field.is_null() {
                            return;
                        }
                        let child_payload = *field;
                        if child_payload.is_null() {
                            return;
                        }
                        let child_header = GcObjectHeader::from_payload(child_payload);
                        if child_header.is_null() {
                            return;
                        }

                        if !(*child_header).forwarding_ptr.is_null() {
                            *field = (*child_header).forwarding_ptr;
                            return;
                        }

                        if (*child_header).generation() >= 2 {
                            return;
                        }

                        let new_header = self.copy_or_promote_young(child_header);
                        if !new_header.is_null() {
                            let new_payload = (*new_header).payload();
                            (*child_header).forwarding_ptr = new_payload;
                            *field = new_payload;
                            worklist.push(new_header);
                        }
                    });
                }
            }
        }

        let mut survivors = Vec::new();
        let mut new_ages = HashMap::new();
        for &original in &young_objects {
            unsafe {
                let fwd = (*original).forwarding_ptr;
                if !fwd.is_null() {
                    // Object was copied (either within young or promoted to old).
                    let new_header = GcObjectHeader::from_payload(fwd);
                    if (*new_header).generation() >= 2 {
                        self.old.add_object(new_header);
                    } else {
                        survivors.push(new_header);
                        let age = self.young.age_of(original);
                        new_ages.insert(new_header, age.saturating_add(1));
                    }
                    ruyi_dealloc((*original).payload());
                } else if (*original).is_pinned() {
                    // Pinned root — keep in place and age.
                    let age = self.young.age_of(original).saturating_add(1);
                    if age >= self.young.promotion_threshold() {
                        (*original).set_generation(2);
                        self.old.add_object(original);
                    } else {
                        survivors.push(original);
                        new_ages.insert(original, age);
                    }
                } else {
                    // Unreachable — reclaim.
                    ruyi_dealloc((*original).payload());
                }
            }
        }

        self.young.replace(survivors);
        self.young.set_ages(new_ages);
        self.young_collections.fetch_add(1, Ordering::Relaxed);
    }

    /// Run a full collection (both young and old generations).
    pub fn collect_full(&self) {
        if !self.enabled {
            return;
        }

        let young_objects = self.young.objects();
        let old_objects = self.old.objects();
        let roots = self.roots.all_roots();

        for &header in &young_objects {
            unsafe {
                (*header).clear_marked();
                (*header).forwarding_ptr = std::ptr::null_mut();
            }
        }
        for &header in &old_objects {
            unsafe {
                (*header).clear_marked();
                (*header).forwarding_ptr = std::ptr::null_mut();
            }
        }

        let mut marked = HashSet::new();
        for &root in &roots {
            unsafe { self.mark_recursive(root, &mut marked) };
        }

        let mut new_young = Vec::new();
        let mut new_old = Vec::new();
        let mut new_ages = HashMap::new();

        for &header in &young_objects {
            unsafe {
                if (*header).is_marked() {
                    if (*header).is_pinned() {
                        new_young.push(header);
                        new_ages.insert(header, self.young.age_of(header));
                    } else {
                        let size = (*header).size;
                        let type_info = (*header).type_info;
                        let new_ptr = ruyi_alloc(size, type_info, MemoryStrategy::GC);
                        if !new_ptr.is_null() {
                            let new_header = GcObjectHeader::from_payload(new_ptr);
                            let age = self.young.age_of(header).saturating_add(1);
                            if age >= self.young.promotion_threshold() {
                                (*new_header).set_generation(2);
                                new_old.push(new_header);
                            } else {
                                (*new_header).set_generation(1);
                                new_young.push(new_header);
                                new_ages.insert(new_header, age);
                            }
                            std::ptr::copy_nonoverlapping((*header).payload(), new_ptr, size);
                            (*header).forwarding_ptr = new_ptr;
                        }
                    }
                }
            }
        }

        for &header in &old_objects {
            unsafe {
                if (*header).is_marked() {
                    if (*header).is_pinned() {
                        new_old.push(header);
                    } else {
                        let size = (*header).size;
                        let type_info = (*header).type_info;
                        let new_ptr = ruyi_alloc(size, type_info, MemoryStrategy::GC);
                        if !new_ptr.is_null() {
                            let new_header = GcObjectHeader::from_payload(new_ptr);
                            (*new_header).set_generation(2);
                            std::ptr::copy_nonoverlapping((*header).payload(), new_ptr, size);
                            (*header).forwarding_ptr = new_ptr;
                            new_old.push(new_header);
                        }
                    }
                }
            }
        }

        for &root in &roots {
            unsafe { self.update_references(root) };
        }
        for &header in &new_young {
            unsafe { self.update_references(header) };
        }
        for &header in &new_old {
            unsafe { self.update_references(header) };
        }

        for &header in &young_objects {
            unsafe {
                if !(*header).is_pinned() {
                    ruyi_dealloc((*header).payload());
                }
            }
        }
        for &header in &old_objects {
            unsafe {
                if !(*header).is_pinned() {
                    ruyi_dealloc((*header).payload());
                }
            }
        }

        self.young.replace(new_young);
        self.young.set_ages(new_ages);
        self.old.replace(new_old);
        self.barrier.clear();
        self.full_collections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn collect(&self) {
        self.collect_young();
    }

    pub fn object_count(&self) -> usize {
        self.young.object_count() + self.old.object_count()
    }

    pub fn young_object_count(&self) -> usize {
        self.young.object_count()
    }

    pub fn old_object_count(&self) -> usize {
        self.old.object_count()
    }

    pub fn root_count(&self) -> usize {
        self.roots.stack_count() + self.roots.global_count()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn young_collection_count(&self) -> usize {
        self.young_collections.load(Ordering::Relaxed)
    }

    pub fn full_collection_count(&self) -> usize {
        self.full_collections.load(Ordering::Relaxed)
    }

    pub fn write_barrier(&self, obj: *mut GcObjectHeader, field: *mut *mut u8, new_value: *mut u8) {
        unsafe { self.barrier.write_field(obj, new_value) };
        if !field.is_null() {
            unsafe { *field = new_value };
        }
    }

    pub fn barrier(&self) -> &WriteBarrier {
        &self.barrier
    }

    pub fn roots(&self) -> &RootSet {
        &self.roots
    }

    unsafe fn copy_or_promote_young(&self, header: *mut GcObjectHeader) -> *mut GcObjectHeader {
        let age = self.young.age_of(header).saturating_add(1);
        let size = (*header).size;
        let type_info = (*header).type_info;

        if age >= self.young.promotion_threshold() {
            let new_ptr = ruyi_alloc(size, type_info, MemoryStrategy::GC);
            if new_ptr.is_null() {
                return std::ptr::null_mut();
            }
            let new_header = GcObjectHeader::from_payload(new_ptr);
            (*new_header).set_generation(2);
            std::ptr::copy_nonoverlapping((*header).payload(), new_ptr, size);
            new_header
        } else {
            let new_ptr = ruyi_alloc(size, type_info, MemoryStrategy::GC);
            if new_ptr.is_null() {
                return std::ptr::null_mut();
            }
            let new_header = GcObjectHeader::from_payload(new_ptr);
            (*new_header).set_generation(1);
            std::ptr::copy_nonoverlapping((*header).payload(), new_ptr, size);
            self.young.set_age(new_header, age);
            new_header
        }
    }

    unsafe fn mark_recursive(&self, start: *mut GcObjectHeader, marked: &mut HashSet<*mut GcObjectHeader>) {
        let mut worklist: Vec<*mut GcObjectHeader> = vec![start];

        while let Some(header) = worklist.pop() {
            if header.is_null() || marked.contains(&header) {
                continue;
            }
            if (*header).is_marked() {
                continue;
            }
            (*header).set_marked();
            marked.insert(header);

            let type_info = (*header).type_info;
            if !type_info.is_null() {
                if let Some(trace) = (*type_info).trace_fn {
                    let payload = (*header).payload();
                    trace(payload, &mut |field: *mut *mut u8| {
                        if !field.is_null() {
                            let child = *field;
                            if !child.is_null() {
                                let child_header = GcObjectHeader::from_payload(child);
                                worklist.push(child_header);
                            }
                        }
                    });
                }
            }
        }
    }

    unsafe fn update_references(&self, header: *mut GcObjectHeader) {
        if header.is_null() {
            return;
        }
        let type_info = (*header).type_info;
        if type_info.is_null() {
            return;
        }
        if let Some(trace) = (*type_info).trace_fn {
            let payload = (*header).payload();
            trace(payload, &mut |field: *mut *mut u8| {
                if !field.is_null() {
                    let child = *field;
                    if !child.is_null() {
                        let child_header = GcObjectHeader::from_payload(child);
                        if !(*child_header).forwarding_ptr.is_null() {
                            *field = (*child_header).forwarding_ptr;
                        }
                    }
                }
            });
        }
    }
}

impl Default for GenerationalCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_int_type() -> *mut TypeInfo {
        static mut INT_TYPE: TypeInfo = TypeInfo {
            type_id: 1,
            type_name: "int",
            destructor: None,
            trace_fn: None,
        };
        &raw mut INT_TYPE
    }

    fn make_pair_type() -> *mut TypeInfo {
        unsafe fn trace_pair(payload: *mut u8, cb: &mut dyn FnMut(*mut *mut u8)) {
            let left = payload as *mut *mut u8;
            let right = left.add(1);
            if !(*left).is_null() {
                cb(left);
            }
            if !(*right).is_null() {
                cb(right);
            }
        }

        static mut PAIR_TYPE: TypeInfo = TypeInfo {
            type_id: 2,
            type_name: "pair",
            destructor: None,
            trace_fn: Some(trace_pair),
        };
        &raw mut PAIR_TYPE
    }

    #[test]
    fn test_alloc_and_minor_collect() {
        let gc = GenerationalCollector::new();

        unsafe {
            let ptr = gc.allocate(8, make_int_type());
            assert!(!ptr.is_null());
            assert_eq!(gc.young_object_count(), 1);
            assert_eq!(gc.old_object_count(), 0);

            gc.add_root(ptr);
            gc.collect_young();
            assert_eq!(gc.young_object_count(), 1);
            assert_eq!(gc.old_object_count(), 0);

            gc.remove_root(ptr);
            gc.collect_young();
            assert_eq!(gc.young_object_count(), 0);
            assert_eq!(gc.old_object_count(), 0);
        }
    }

    #[test]
    fn test_nested_trace_minor_collect() {
        let gc = GenerationalCollector::new();

        unsafe {
            let a = gc.allocate(8, make_int_type());
            let b = gc.allocate(8, make_int_type());
            let pair = gc.allocate(16, make_pair_type());

            *(pair as *mut *mut u8) = a;
            *((pair as *mut *mut u8).add(1)) = b;

            gc.add_root(pair);
            gc.collect_young();
            assert_eq!(gc.object_count(), 3);

            *((pair as *mut *mut u8).add(1)) = std::ptr::null_mut();
            gc.collect_young();
            assert_eq!(gc.object_count(), 2);

            gc.remove_root(pair);
            gc.collect_young();
            assert_eq!(gc.object_count(), 0);
        }
    }

    #[test]
    fn test_promotion_after_threshold() {
        let gc = GenerationalCollector::with_threshold(2);

        unsafe {
            let ptr = gc.allocate(8, make_int_type());
            gc.add_root(ptr);

            gc.collect_young();
            assert_eq!(gc.young_object_count(), 1);
            assert_eq!(gc.old_object_count(), 0);

            gc.collect_young();
            assert_eq!(gc.young_object_count(), 0);
            assert_eq!(gc.old_object_count(), 1);

            gc.remove_root(ptr);
            gc.collect_full();
            assert_eq!(gc.object_count(), 0);
        }
    }

    #[test]
    fn test_write_barrier_tracks_cross_gen() {
        let gc = GenerationalCollector::with_threshold(1);

        unsafe {
            let young = gc.allocate(8, make_int_type());
            let old = gc.allocate(8, make_int_type());
            let old_header = GcObjectHeader::from_payload(old);
            (*old_header).set_generation(2);
            gc.old.add_object(old_header);

            gc.write_barrier(old_header, (old as *mut *mut u8).add(1), young);
            assert_eq!(gc.barrier().len(), 1);
        }
    }

    #[test]
    fn test_full_collect_compacts_old() {
        let gc = GenerationalCollector::new();

        unsafe {
            let a = gc.allocate(8, make_int_type());
            let b = gc.allocate(8, make_int_type());
            gc.add_root(a);
            gc.add_root(b);

            gc.collect_full();
            assert_eq!(gc.object_count(), 2);

            gc.remove_root(a);
            gc.collect_full();
            assert_eq!(gc.object_count(), 1);

            gc.remove_root(b);
            gc.collect_full();
            assert_eq!(gc.object_count(), 0);
        }
    }

    #[test]
    fn test_disabled_collection() {
        let mut gc = GenerationalCollector::new();
        unsafe {
            let ptr = gc.allocate(8, make_int_type());
            gc.add_root(ptr);
            gc.set_enabled(false);
            gc.remove_root(ptr);
            gc.collect_young();
            assert_eq!(gc.object_count(), 1);
        }
    }

    #[test]
    fn test_stress_many_objects() {
        let gc = GenerationalCollector::with_threshold(3);

        unsafe {
            let int_type = make_int_type();
            let pair_type = make_pair_type();

            const N: usize = 10_000;
            let mut nodes: Vec<*mut u8> = Vec::with_capacity(N);

            for i in 0..N {
                let node = gc.allocate(16, pair_type);
                let value = gc.allocate(8, int_type);
                *(value as *mut u64) = i as u64;
                *(node as *mut *mut u8) = value;

                if i > 0 {
                    *((node as *mut *mut u8).add(1)) = nodes[i - 1];
                } else {
                    *((node as *mut *mut u8).add(1)) = std::ptr::null_mut();
                }

                nodes.push(node);
            }

            gc.add_root(nodes[N - 1]);

            for _ in 0..5 {
                gc.collect_young();
            }

            assert!(gc.object_count() > 0);
            gc.collect_full();
            assert_eq!(gc.object_count(), N * 2);

            gc.remove_root(nodes[N - 1]);
            gc.collect_full();
            assert_eq!(gc.object_count(), 0);
        }
    }

    #[test]
    fn test_remembered_set_in_minor_collect() {
        let gc = GenerationalCollector::with_threshold(5);

        unsafe {
            let young = gc.allocate(8, make_int_type());
            let old = gc.allocate(16, make_pair_type());
            let old_header = GcObjectHeader::from_payload(old);
            (*old_header).set_generation(2);
            gc.old.add_object(old_header);
            gc.barrier().add_to_remembered_set(old_header);

            gc.write_barrier(old_header, (old as *mut *mut u8).add(1), young);

            gc.add_root(old);
            gc.collect_young();

            assert!(gc.object_count() >= 2);
        }
    }
}
