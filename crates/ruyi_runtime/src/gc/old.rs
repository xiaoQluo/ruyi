use std::sync::Mutex;

use crate::alloc::{GcObjectHeader, MemoryStrategy, TypeInfo, ruyi_alloc};

/// Old generation for the generational GC.
///
/// Objects that have survived multiple young-generation collections
/// are promoted here.  The old generation is collected less
/// frequently using a mark-compact algorithm.
pub struct OldGeneration {
    objects: Mutex<Vec<*mut GcObjectHeader>>,
}

impl OldGeneration {
    pub fn new() -> Self {
        Self {
            objects: Mutex::new(Vec::new()),
        }
    }

    /// Allocate a new object directly in the old generation.
    ///
    /// # Safety
    /// `type_info` must remain valid for the lifetime of the object.
    pub unsafe fn allocate(&self, size: usize, type_info: *mut TypeInfo) -> *mut u8 {
        let ptr = ruyi_alloc(size, type_info, MemoryStrategy::GC);
        if !ptr.is_null() {
            let header = GcObjectHeader::from_payload(ptr);
            (*header).set_generation(2);
            self.objects.lock().unwrap().push(header);
        }
        ptr
    }

    pub fn object_count(&self) -> usize {
        self.objects.lock().unwrap().len()
    }

    pub fn objects(&self) -> Vec<*mut GcObjectHeader> {
        self.objects.lock().unwrap().clone()
    }

    pub fn replace(&self, new_objects: Vec<*mut GcObjectHeader>) {
        *self.objects.lock().unwrap() = new_objects;
    }

    pub fn clear(&self) {
        self.objects.lock().unwrap().clear();
    }

    pub fn add_object(&self, header: *mut GcObjectHeader) {
        if !header.is_null() {
            unsafe { (*header).set_generation(2) };
            self.objects.lock().unwrap().push(header);
        }
    }
}

impl Default for OldGeneration {
    fn default() -> Self {
        Self::new()
    }
}
