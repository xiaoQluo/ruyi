use std::alloc::GlobalAlloc;
use std::collections::HashSet;
use std::sync::Mutex;

use crate::alloc::{GcObjectHeader, Heap, MemoryStrategy, TypeInfo};

pub mod barrier;
pub mod generational;
pub mod old;
pub mod roots;
pub mod young;

pub use generational::GenerationalCollector;

/// Simple mark-and-sweep collector used as the baseline GC.
///
/// This implementation is kept for backward compatibility and benchmarking.
/// New code should prefer `GenerationalCollector`.
pub struct MarkSweepCollector {
    #[allow(dead_code)]
    heap: Heap,
    /// All objects currently allocated from this collector.
    objects: Mutex<Vec<*mut GcObjectHeader>>,
    /// Externally registered GC roots (stack frames, globals, etc.).
    roots: Mutex<Vec<*mut GcObjectHeader>>,
    /// Whether collection is enabled.
    enabled: bool,
}

impl MarkSweepCollector {
    /// Create a new mark-and-sweep collector backed by the system heap.
    pub fn new() -> Self {
        Self {
            heap: Heap::new(),
            objects: Mutex::new(Vec::new()),
            roots: Mutex::new(Vec::new()),
            enabled: true,
        }
    }

    /// Allocate a GC-managed object with the given payload size.
    ///
    /// The returned pointer points to the **payload**. Use
    /// `GcObjectHeader::from_payload` to reach the header.
    ///
    /// # Safety
    ///
    /// `type_info` must remain valid for the lifetime of the object.
    pub unsafe fn allocate(&self, size: usize, type_info: *mut TypeInfo) -> *mut u8 {
        let ptr = super::alloc::ruyi_alloc(size, type_info, MemoryStrategy::GC);
        if !ptr.is_null() {
            let header = GcObjectHeader::from_payload(ptr);
            self.objects.lock().unwrap().push(header);
        }
        ptr
    }

    /// Register a pointer as a GC root.
    ///
    /// Roots are not collected and serve as the starting points for
    /// the mark phase.
    ///
    /// # Safety
    ///
    /// `ptr` must point to the payload of a valid GC object.
    pub unsafe fn add_root(&self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }
        let header = GcObjectHeader::from_payload(ptr);
        self.roots.lock().unwrap().push(header);
    }

    /// Unregister a previously registered root.
    ///
    /// # Safety
    ///
    /// `ptr` must have been previously passed to `add_root`.
    pub unsafe fn remove_root(&self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }
        let header = GcObjectHeader::from_payload(ptr);
        let mut roots = self.roots.lock().unwrap();
        if let Some(pos) = roots.iter().position(|&h| h == header) {
            roots.swap_remove(pos);
        }
    }

    /// Return the number of tracked objects.
    pub fn object_count(&self) -> usize {
        self.objects.lock().unwrap().len()
    }

    /// Return the number of registered roots.
    pub fn root_count(&self) -> usize {
        self.roots.lock().unwrap().len()
    }

    /// Check whether `ptr` is the payload pointer of an object currently
    /// tracked by this collector.
    pub fn is_valid_payload(&self, ptr: *mut u8) -> bool {
        if ptr.is_null() {
            return false;
        }
        let header_addr = (ptr as usize).wrapping_sub(std::mem::size_of::<GcObjectHeader>())
            as *mut GcObjectHeader;
        let objects = self.objects.lock().unwrap();
        objects.contains(&header_addr)
    }

    /// Enable or disable collection.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Run a full mark-and-sweep collection.
    ///
    /// 1. **Clear** all mark bits.
    /// 2. **Mark** from every registered root (recursive via `trace_fn`).
    /// 3. **Sweep** unmarked objects.
    pub fn collect(&self) {
        if !self.enabled {
            return;
        }

        let mut objects = self.objects.lock().unwrap();
        if objects.is_empty() {
            return;
        }

        // Phase 1 — clear marks.
        for &obj in objects.iter() {
            unsafe { (*obj).clear_marked() };
        }

        // Phase 2 — mark from roots.
        let roots = self.roots.lock().unwrap().clone();
        let mut marked = HashSet::new();
        for &root in &roots {
            self.mark_object(root, &mut marked);
        }

        // Phase 3 — sweep unmarked objects.
        let mut i = 0;
        while i < objects.len() {
            let obj = objects[i];
            if unsafe { (*obj).is_marked() } {
                i += 1;
            } else {
                // Object is unreachable — destroy and free.
                let ptr = unsafe { (*obj).payload() };
                unsafe { super::alloc::ruyi_dealloc(ptr) };
                objects.swap_remove(i);
            }
        }
    }

    fn mark_object(&self, header: *mut GcObjectHeader, marked: &mut HashSet<*mut GcObjectHeader>) {
        if header.is_null() || marked.contains(&header) {
            return;
        }
        unsafe {
            if (*header).is_marked() {
                return;
            }
            (*header).set_marked();
        }
        marked.insert(header);

        // Trace interior pointers.
        unsafe {
            let type_info = (*header).type_info;
            if !type_info.is_null() {
                if let Some(trace) = (*type_info).trace_fn {
                    let payload = (*header).payload();
                    let mut callback = |field: *mut *mut u8| {
                        if field.is_null() {
                            return;
                        }
                        let child = *field;
                        if !child.is_null() {
                            let child_header = GcObjectHeader::from_payload(child);
                            self.mark_object(child_header, marked);
                        }
                    };
                    trace(payload, &mut callback);
                }
            }
        }
    }
}

impl Default for MarkSweepCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Primary collector that wraps the generational implementation.
///
/// This struct replaces the old `MarkSweepCollector` as the default
/// collector used by the Ruyi runtime.
pub struct Collector {
    inner: GenerationalCollector,
}

impl Collector {
    pub fn new() -> Self {
        Self {
            inner: GenerationalCollector::new(),
        }
    }

    pub fn collect(&self) {
        self.inner.collect();
    }
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

/// Legacy struct kept for API compatibility with the initial scaffold.
///
/// New code should use `MarkSweepCollector::allocate`.
pub struct GcAllocator;

impl Default for GcAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl GcAllocator {
    pub fn new() -> Self {
        Self
    }

    pub fn allocate(&self, layout: std::alloc::Layout) -> *mut u8 {
        unsafe { std::alloc::System.alloc(layout) }
    }

    /// # Safety
    /// `ptr` must be a valid, non-null pointer previously returned by [`Self::allocate`]
    /// with the same `layout`.
    pub unsafe fn deallocate(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }

    /// # Safety
    /// `ptr` must be a valid, non-null pointer previously returned by [`Self::allocate`]
    /// with `old_layout`. The original buffer is deallocated on success.
    pub unsafe fn reallocate(
        &self,
        ptr: *mut u8,
        old_layout: std::alloc::Layout,
        new_layout: std::alloc::Layout,
    ) -> *mut u8 {
        if new_layout.size() <= old_layout.size() {
            return ptr;
        }
        let new_ptr = self.allocate(new_layout);
        unsafe {
            std::ptr::copy_nonoverlapping(ptr, new_ptr, old_layout.size());
            self.deallocate(ptr, old_layout);
        }
        new_ptr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_and_collect() {
        static mut TYPE_INFO: TypeInfo = TypeInfo {
            type_id: 10,
            type_name: "int_box",
            destructor: None,
            trace_fn: None,
        };

        let collector = MarkSweepCollector::new();

        unsafe {
            let ptr = collector.allocate(8, &raw mut TYPE_INFO);
            assert!(!ptr.is_null());
            assert_eq!(collector.object_count(), 1);

            collector.add_root(ptr);
            assert_eq!(collector.root_count(), 1);

            // Object is rooted — should survive collection.
            collector.collect();
            assert_eq!(collector.object_count(), 1);

            collector.remove_root(ptr);
            assert_eq!(collector.root_count(), 0);

            // No more roots — object should be reclaimed.
            collector.collect();
            assert_eq!(collector.object_count(), 0);
        }
    }

    #[test]
    fn test_nested_trace() {
        unsafe fn trace_pair(payload: *mut u8, cb: &mut dyn FnMut(*mut *mut u8)) {
            // Payload layout: [left: *mut u8; right: *mut u8]
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
            type_id: 20,
            type_name: "pair",
            destructor: None,
            trace_fn: Some(trace_pair),
        };

        static mut INT_TYPE: TypeInfo = TypeInfo {
            type_id: 21,
            type_name: "int",
            destructor: None,
            trace_fn: None,
        };

        let collector = MarkSweepCollector::new();

        unsafe {
            let a = collector.allocate(8, &raw mut INT_TYPE);
            let b = collector.allocate(8, &raw mut INT_TYPE);
            let pair = collector.allocate(16, &raw mut PAIR_TYPE);

            *(a as *mut u64) = 1;
            *(b as *mut u64) = 2;
            *(pair as *mut *mut u8) = a;
            *((pair as *mut *mut u8).add(1)) = b;

            collector.add_root(pair);
            collector.collect();
            assert_eq!(collector.object_count(), 3);

            // Break the link from pair -> b and remove root from b.
            *((pair as *mut *mut u8).add(1)) = std::ptr::null_mut();
            collector.remove_root(a);
            collector.remove_root(b);

            collector.collect();
            // pair and a survive (a is still reachable via pair), b is dead.
            assert_eq!(collector.object_count(), 2);

            collector.remove_root(pair);
            collector.collect();
            assert_eq!(collector.object_count(), 0);
        }
    }

    #[test]
    fn test_collect_disabled() {
        static mut TYPE_INFO: TypeInfo = TypeInfo {
            type_id: 30,
            type_name: "dummy",
            destructor: None,
            trace_fn: None,
        };

        let mut collector = MarkSweepCollector::new();
        unsafe {
            let ptr = collector.allocate(8, &raw mut TYPE_INFO);
            collector.add_root(ptr);
            collector.set_enabled(false);
            collector.remove_root(ptr);
            collector.collect();
            assert_eq!(collector.object_count(), 1); // not swept because disabled
        }
    }
}
