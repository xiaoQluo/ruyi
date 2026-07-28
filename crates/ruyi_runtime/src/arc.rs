/**
 * Automatic Reference Counting (ARC) runtime for Ruyi.
 *
 * Provides optional ARC memory management as an alternative to GC.
 * ARC objects use the same GcObjectHeader but with MemoryStrategy::ARC.
 *
 * Features:
 * - Explicit retain/release with atomic reference counting
 * - Weak reference support via a side table
 * - Basic cycle detection using trial-delete algorithm
 * - ARC/GC boundary handling
 *
 * @author Ruyi Team
 * @date 2026-05-02
 */
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;

use crate::alloc::{ruyi_alloc, ruyi_dealloc, GcObjectHeader, MemoryStrategy, TypeInfo};

static WEAK_TABLE: OnceLock<Mutex<WeakTable>> = OnceLock::new();

fn get_weak_table() -> &'static Mutex<WeakTable> {
    WEAK_TABLE.get_or_init(|| Mutex::new(WeakTable::new()))
}

unsafe impl Send for WeakTable {}
unsafe impl Sync for WeakTable {}

/// A weak reference to an ARC object.
///
/// When the object is deallocated, the weak reference becomes `None`.
#[derive(Debug)]
pub struct WeakRef {
    /// The payload pointer, or null if the object has been deallocated.
    #[allow(dead_code)]
    ptr: *mut u8,
    /// Unique slot ID in the weak table.
    slot_id: u64,
}

unsafe impl Send for WeakRef {}
unsafe impl Sync for WeakRef {}

/// Internal weak table state.
pub struct WeakTable {
    next_id: u64,
    slots: HashMap<u64, *mut u8>,
    object_slots: HashMap<*mut u8, Vec<u64>>,
}

impl Default for WeakTable {
    fn default() -> Self {
        Self::new()
    }
}

impl WeakTable {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            slots: HashMap::new(),
            object_slots: HashMap::new(),
        }
    }

    fn allocate_slot(&mut self, ptr: *mut u8) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.slots.insert(id, ptr);
        self.object_slots.entry(ptr).or_default().push(id);
        id
    }

    fn invalidate_object(&mut self, ptr: *mut u8) {
        if let Some(ids) = self.object_slots.remove(&ptr) {
            for id in ids {
                self.slots.insert(id, std::ptr::null_mut());
            }
        }
    }

    fn get(&self, slot_id: u64) -> *mut u8 {
        self.slots
            .get(&slot_id)
            .copied()
            .unwrap_or(std::ptr::null_mut())
    }

    fn remove_slot(&mut self, slot_id: u64) {
        if let Some(ptr) = self.slots.remove(&slot_id) {
            if !ptr.is_null() {
                if let Some(ids) = self.object_slots.get_mut(&ptr) {
                    ids.retain(|&id| id != slot_id);
                }
            }
        }
    }
}

/// Allocate a new ARC-managed object.
///
/// The returned pointer points to the **payload**. The initial reference
/// count is 1.
///
/// # Safety
///
/// `type_info` must remain valid for the lifetime of the object.
#[no_mangle]
pub unsafe extern "C" fn ruyi_arc_alloc(size: usize, type_info: *mut TypeInfo) -> *mut u8 {
    ruyi_alloc(size, type_info, MemoryStrategy::ARC)
}

/// Atomically increment the reference count of an ARC object.
///
/// # Safety
///
/// `ptr` must be a valid payload pointer returned by `ruyi_arc_alloc`.
#[no_mangle]
pub unsafe extern "C" fn ruyi_arc_retain(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    let header = GcObjectHeader::from_payload(ptr);
    (*header).retain();
}

/// Atomically decrement the reference count of an ARC object.
///
/// If the reference count reaches zero, the destructor is called and
/// the memory is freed. All weak references are invalidated.
///
/// # Safety
///
/// `ptr` must be a valid payload pointer returned by `ruyi_arc_alloc`.
#[no_mangle]
pub unsafe extern "C" fn ruyi_arc_release(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    let header = GcObjectHeader::from_payload(ptr);
    let new_count = (*header).release();
    if new_count == 0 {
        let mut table = get_weak_table().lock().unwrap();
        table.invalidate_object(ptr);
        drop(table);
        ruyi_dealloc(ptr);
    }
}

/// Return the current reference count of an ARC object.
///
/// # Safety
///
/// `ptr` must be a valid payload pointer.
pub unsafe fn ruyi_arc_ref_count(ptr: *mut u8) -> u32 {
    if ptr.is_null() {
        return 0;
    }
    let header = GcObjectHeader::from_payload(ptr);
    (*header).ref_count()
}

/// Create a weak reference to an ARC object.
///
/// The weak reference does not keep the object alive. Use
/// `ruyi_arc_weak_load` to attempt to obtain a strong reference.
///
/// # Safety
///
/// `ptr` must be a valid payload pointer.
pub unsafe fn ruyi_arc_weak(ptr: *mut u8) -> WeakRef {
    if ptr.is_null() {
        return WeakRef {
            ptr: std::ptr::null_mut(),
            slot_id: 0,
        };
    }
    let mut table = get_weak_table().lock().unwrap();
    let slot_id = table.allocate_slot(ptr);
    WeakRef { ptr, slot_id }
}

/// Attempt to load a strong reference from a weak reference.
///
/// Returns the object pointer if it is still alive, or `null_mut` if
/// the object has been deallocated.
///
/// # Safety
///
/// The returned pointer, if non-null, is a valid ARC object with an
/// incremented reference count. The caller must release it.
pub unsafe fn ruyi_arc_weak_load(weak: &WeakRef) -> *mut u8 {
    let table = get_weak_table().lock().unwrap();
    let ptr = table.get(weak.slot_id);
    drop(table);
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    // Increment refcount before returning.
    let header = GcObjectHeader::from_payload(ptr);
    (*header).retain();
    ptr
}

/// Drop a weak reference slot.
///
/// # Safety
///
/// Must be called exactly once for each weak reference created by
/// `ruyi_arc_weak`.
pub unsafe fn ruyi_arc_weak_drop(weak: WeakRef) {
    if weak.slot_id == 0 {
        return;
    }
    let mut table = get_weak_table().lock().unwrap();
    table.remove_slot(weak.slot_id);
}

/// Check whether a pointer refers to an ARC-managed object.
///
/// # Safety
///
/// `ptr` must be a valid payload pointer.
pub unsafe fn ruyi_is_arc(ptr: *mut u8) -> bool {
    if ptr.is_null() {
        return false;
    }
    let header = GcObjectHeader::from_payload(ptr);
    (*header).strategy() == MemoryStrategy::ARC
}

/// Check whether a pointer refers to a GC-managed object.
///
/// # Safety
///
/// `ptr` must be a valid payload pointer.
pub unsafe fn ruyi_is_gc(ptr: *mut u8) -> bool {
    if ptr.is_null() {
        return false;
    }
    let header = GcObjectHeader::from_payload(ptr);
    (*header).strategy() == MemoryStrategy::GC
}

// ── Cycle Detection ──────────────────────────────────────────

/// Cycle detector for ARC objects using a simplified trial-delete algorithm.
///
/// The algorithm works as follows:
/// 1. For a suspected root object, temporarily decrement the refcount
///    of all objects reachable from it.
/// 2. If the root's refcount drops to zero, it is part of a cycle.
/// 3. Restore refcounts by re-incrementing.
/// 4. Actually release the cycle members.
///
/// This is a simplified version that requires the caller to provide
/// a tracing function for each object type.
pub struct CycleDetector {
    /// Objects currently being evaluated in the trial-delete phase.
    candidate_set: Vec<*mut GcObjectHeader>,
}

impl CycleDetector {
    pub fn new() -> Self {
        Self {
            candidate_set: Vec::new(),
        }
    }

    /// Attempt to detect and break a cycle starting from `root`.
    ///
    /// `trace_fn` is called for each object and must invoke the callback
    /// for every interior pointer field.
    ///
    /// Returns `true` if a cycle was detected and broken.
    ///
    /// # Safety
    ///
    /// All pointers must be valid ARC object payload pointers.
    pub unsafe fn detect_and_break(
        &mut self,
        root: *mut u8,
        trace_fn: unsafe fn(*mut u8, &mut dyn FnMut(*mut *mut u8)),
    ) -> bool {
        if root.is_null() {
            return false;
        }

        self.candidate_set.clear();

        // Phase 1: Collect all reachable objects into candidate_set.
        self.collect_reachable(root, trace_fn);

        if self.candidate_set.len() <= 1 {
            // No possible cycle with only one object.
            return false;
        }

        // Phase 2: Trial-delete — decrement refcount of each candidate
        // for each interior pointer from another candidate.
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..self.candidate_set.len() {
                let header = self.candidate_set[i];
                let payload = (*header).payload();
                let mut callback = |field: *mut *mut u8| {
                    if field.is_null() {
                        return;
                    }
                    let child = *field;
                    if child.is_null() {
                        return;
                    }
                    let child_header = GcObjectHeader::from_payload(child);
                    if self.candidate_set.contains(&child_header) && (*child_header).ref_count() > 0
                    {
                        (*child_header).release();
                        changed = true;
                    }
                };
                trace_fn(payload, &mut callback);
            }
        }

        // Phase 3: Check which objects have refcount == 0.
        let cycle_members: Vec<*mut GcObjectHeader> = self
            .candidate_set
            .iter()
            .filter(|&&h| (*h).ref_count() == 0)
            .copied()
            .collect();

        // Phase 4: Restore refcounts for cycle members from internal refs.
        for &header in &cycle_members {
            let payload = (*header).payload();
            let mut callback = |field: *mut *mut u8| {
                if field.is_null() {
                    return;
                }
                let child = *field;
                if child.is_null() {
                    return;
                }
                let child_header = GcObjectHeader::from_payload(child);
                if cycle_members.contains(&child_header) {
                    (*child_header).retain();
                }
            };
            trace_fn(payload, &mut callback);
        }

        if cycle_members.is_empty() {
            return false;
        }

        // Phase 5: Break the cycle by nulling interior pointers and releasing.
        for &header in &cycle_members {
            let payload = (*header).payload();
            let mut callback = |field: *mut *mut u8| {
                if field.is_null() {
                    return;
                }
                let child = *field;
                if child.is_null() {
                    return;
                }
                let child_header = GcObjectHeader::from_payload(child);
                if cycle_members.contains(&child_header) {
                    *field = std::ptr::null_mut();
                }
            };
            trace_fn(payload, &mut callback);
        }

        for &header in &cycle_members {
            let payload = (*header).payload();
            ruyi_arc_release(payload);
        }

        true
    }

    unsafe fn collect_reachable(
        &mut self,
        root: *mut u8,
        trace_fn: unsafe fn(*mut u8, &mut dyn FnMut(*mut *mut u8)),
    ) {
        let root_header = GcObjectHeader::from_payload(root);
        if self.candidate_set.contains(&root_header) {
            return;
        }
        self.candidate_set.push(root_header);

        let payload = (*root_header).payload();
        let mut callback = |field: *mut *mut u8| {
            if field.is_null() {
                return;
            }
            let child = *field;
            if child.is_null() {
                return;
            }
            if ruyi_is_arc(child) {
                self.collect_reachable(child, trace_fn);
            }
        };
        trace_fn(payload, &mut callback);
    }
}

impl Default for CycleDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ── ARC/GC Boundary ──────────────────────────────────────────

/// Retain a pointer regardless of whether it is ARC or GC.
///
/// For ARC objects, increments the refcount.
/// For GC objects, registers it as a root.
///
/// # Safety
///
/// `ptr` must be a valid payload pointer.
pub unsafe fn ruyi_retain_any(ptr: *mut u8, gc_roots: &mut Vec<*mut u8>) {
    if ptr.is_null() {
        return;
    }
    if ruyi_is_arc(ptr) {
        ruyi_arc_retain(ptr);
    } else {
        gc_roots.push(ptr);
    }
}

/// Release a pointer regardless of whether it is ARC or GC.
///
/// For ARC objects, decrements the refcount.
/// For GC objects, removes it from the root set.
///
/// # Safety
///
/// `ptr` must be a valid payload pointer.
pub unsafe fn ruyi_release_any(ptr: *mut u8, gc_roots: &mut Vec<*mut u8>) {
    if ptr.is_null() {
        return;
    }
    if ruyi_is_arc(ptr) {
        ruyi_arc_release(ptr);
    } else if let Some(pos) = gc_roots.iter().position(|&p| p == ptr) {
        gc_roots.swap_remove(pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_alloc_release() {
        static mut TYPE_INFO: TypeInfo = TypeInfo {
            type_id: 100,
            type_name: "arc_test",
            destructor: None,
            trace_fn: None,
        };

        unsafe {
            let ptr = ruyi_arc_alloc(16, &raw mut TYPE_INFO);
            assert!(!ptr.is_null());
            assert_eq!(ruyi_arc_ref_count(ptr), 1);

            ruyi_arc_retain(ptr);
            assert_eq!(ruyi_arc_ref_count(ptr), 2);

            ruyi_arc_release(ptr);
            assert_eq!(ruyi_arc_ref_count(ptr), 1);

            ruyi_arc_release(ptr);
        }
    }

    #[test]
    fn test_weak_reference() {
        static mut TYPE_INFO: TypeInfo = TypeInfo {
            type_id: 101,
            type_name: "weak_test",
            destructor: None,
            trace_fn: None,
        };

        unsafe {
            let ptr = ruyi_arc_alloc(8, &raw mut TYPE_INFO);
            let weak = ruyi_arc_weak(ptr);

            // Object still alive
            let loaded = ruyi_arc_weak_load(&weak);
            assert!(!loaded.is_null());
            assert_eq!(ruyi_arc_ref_count(ptr), 2);
            ruyi_arc_release(loaded); // release the strong ref from load

            // Release the original reference
            ruyi_arc_release(ptr);

            // Now weak load should return null
            let loaded2 = ruyi_arc_weak_load(&weak);
            assert!(loaded2.is_null());

            ruyi_arc_weak_drop(weak);
        }
    }

    #[test]
    fn test_cycle_detection_simple() {
        unsafe fn trace_node(payload: *mut u8, cb: &mut dyn FnMut(*mut *mut u8)) {
            let next = payload as *mut *mut u8;
            if !(*next).is_null() {
                cb(next);
            }
        }

        static mut NODE_TYPE: TypeInfo = TypeInfo {
            type_id: 102,
            type_name: "node",
            destructor: None,
            trace_fn: Some(trace_node),
        };

        unsafe {
            let a = ruyi_arc_alloc(8, &raw mut NODE_TYPE);
            let b = ruyi_arc_alloc(8, &raw mut NODE_TYPE);

            *(a as *mut *mut u8) = b;
            ruyi_arc_retain(b);

            *(b as *mut *mut u8) = a;
            ruyi_arc_retain(a);

            let mut detector = CycleDetector::new();
            let found = detector.detect_and_break(a, trace_node);
            assert!(found, "cycle should be detected");
        }
    }

    #[test]
    fn test_retain_release_any() {
        static mut TYPE_INFO: TypeInfo = TypeInfo {
            type_id: 103,
            type_name: "any_test",
            destructor: None,
            trace_fn: None,
        };

        unsafe {
            let ptr = ruyi_arc_alloc(8, &raw mut TYPE_INFO);
            let mut roots = Vec::new();

            ruyi_retain_any(ptr, &mut roots);
            assert_eq!(ruyi_arc_ref_count(ptr), 2);

            ruyi_release_any(ptr, &mut roots);
            assert_eq!(ruyi_arc_ref_count(ptr), 1);

            ruyi_arc_release(ptr);
        }
    }
}
