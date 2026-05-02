use std::collections::HashMap;
use std::sync::Mutex;

use crate::alloc::{GcObjectHeader, MemoryStrategy, TypeInfo, ruyi_alloc};

/// Young generation (nursery) for the generational GC.
///
/// New objects are allocated here.  A minor collection copies live
/// young objects to survivor space and promotes objects that have
/// survived enough collections to the old generation.
pub struct YoungGeneration {
    objects: Mutex<Vec<*mut GcObjectHeader>>,
    age: Mutex<HashMap<*mut GcObjectHeader, u8>>,
    promotion_threshold: u8,
}

impl YoungGeneration {
    pub fn new() -> Self {
        Self::with_threshold(3)
    }

    pub fn with_threshold(promotion_threshold: u8) -> Self {
        Self {
            objects: Mutex::new(Vec::new()),
            age: Mutex::new(HashMap::new()),
            promotion_threshold,
        }
    }

    pub fn promotion_threshold(&self) -> u8 {
        self.promotion_threshold
    }

    /// Allocate a new object in the young generation (eden).
    ///
    /// # Safety
    /// `type_info` must remain valid for the lifetime of the object.
    pub unsafe fn allocate(&self, size: usize, type_info: *mut TypeInfo) -> *mut u8 {
        let ptr = ruyi_alloc(size, type_info, MemoryStrategy::GC);
        if !ptr.is_null() {
            let header = GcObjectHeader::from_payload(ptr);
            (*header).set_generation(0);
            self.objects.lock().unwrap().push(header);
            self.age.lock().unwrap().insert(header, 0);
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
        self.age.lock().unwrap().clear();
    }

    pub fn age_of(&self, header: *mut GcObjectHeader) -> u8 {
        self.age.lock().unwrap().get(&header).copied().unwrap_or(0)
    }

    pub fn set_age(&self, header: *mut GcObjectHeader, value: u8) {
        self.age.lock().unwrap().insert(header, value);
    }

    pub fn set_ages(&self, ages: HashMap<*mut GcObjectHeader, u8>) {
        *self.age.lock().unwrap() = ages;
    }
}

impl Default for YoungGeneration {
    fn default() -> Self {
        Self::new()
    }
}
