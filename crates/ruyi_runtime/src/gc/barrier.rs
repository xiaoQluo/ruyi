use std::sync::Mutex;

#[cfg(test)]
use crate::alloc::ruyi_dealloc;
use crate::alloc::GcObjectHeader;

/// Write barrier that tracks cross-generational references.
///
/// When an old-generation object is mutated to hold a pointer to a
/// young-generation object, the old object is recorded in the
/// **remembered set**.  This ensures that the old object is treated
/// as an additional root during a minor (young-only) collection.
pub struct WriteBarrier {
    remembered_set: Mutex<Vec<*mut GcObjectHeader>>,
}

impl WriteBarrier {
    pub fn new() -> Self {
        Self {
            remembered_set: Mutex::new(Vec::new()),
        }
    }

    /// Record a potential cross-generation write.
    ///
    /// Call this before (or after) storing `new_value` into a field of
    /// `obj`.  If `obj` is in the old generation and `new_value` points
    /// to a young object, `obj` is added to the remembered set.
    ///
    /// # Safety
    /// Both `obj` and `new_value` must be valid non-dangling pointers
    /// (or null for `new_value`).
    pub unsafe fn write_field(&self, obj: *mut GcObjectHeader, new_value: *mut u8) {
        if obj.is_null() || new_value.is_null() {
            return;
        }
        let obj_gen = (*obj).generation();
        if obj_gen < 2 {
            return;
        }
        let child_header = GcObjectHeader::from_payload(new_value);
        if child_header.is_null() {
            return;
        }
        if (*child_header).generation() < 2 {
            self.add_to_remembered_set(obj);
        }
    }

    /// Explicitly add an old-generation object to the remembered set.
    pub fn add_to_remembered_set(&self, obj: *mut GcObjectHeader) {
        let mut set = self.remembered_set.lock().unwrap();
        if !set.contains(&obj) {
            set.push(obj);
        }
    }

    /// Return a snapshot of the remembered set.
    pub fn remembered_set(&self) -> Vec<*mut GcObjectHeader> {
        self.remembered_set.lock().unwrap().clone()
    }

    /// Clear the remembered set (called after a full collection).
    pub fn clear(&self) {
        self.remembered_set.lock().unwrap().clear();
    }

    /// Remove entries whose objects are no longer old-generation.
    ///
    /// Called after a full collection when some old objects may have
    /// been freed or moved.
    pub unsafe fn scrub(&self) {
        let mut set = self.remembered_set.lock().unwrap();
        set.retain(|&h| !h.is_null() && (*h).generation() >= 2);
    }

    pub fn len(&self) -> usize {
        self.remembered_set.lock().unwrap().len()
    }
}

impl Default for WriteBarrier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc::{ruyi_alloc, MemoryStrategy, TypeInfo};

    #[test]
    fn test_young_to_old_no_barrier() {
        static mut TYPE_INFO: TypeInfo = TypeInfo {
            type_id: 1,
            type_name: "test",
            destructor: None,
            trace_fn: None,
        };

        let barrier = WriteBarrier::new();
        unsafe {
            let young = ruyi_alloc(8, &raw mut TYPE_INFO, MemoryStrategy::GC);
            let old = ruyi_alloc(8, &raw mut TYPE_INFO, MemoryStrategy::GC);
            let old_header = GcObjectHeader::from_payload(old);
            (*old_header).set_generation(2);

            barrier.write_field(GcObjectHeader::from_payload(young), old);
            assert_eq!(barrier.len(), 0);

            ruyi_dealloc(young);
            ruyi_dealloc(old);
        }
    }

    #[test]
    fn test_old_to_young_triggers_barrier() {
        static mut TYPE_INFO: TypeInfo = TypeInfo {
            type_id: 2,
            type_name: "test",
            destructor: None,
            trace_fn: None,
        };

        let barrier = WriteBarrier::new();
        unsafe {
            let young = ruyi_alloc(8, &raw mut TYPE_INFO, MemoryStrategy::GC);
            let old = ruyi_alloc(8, &raw mut TYPE_INFO, MemoryStrategy::GC);
            let old_header = GcObjectHeader::from_payload(old);
            (*old_header).set_generation(2);

            barrier.write_field(old_header, young);
            assert_eq!(barrier.len(), 1);

            ruyi_dealloc(young);
            ruyi_dealloc(old);
        }
    }
}
