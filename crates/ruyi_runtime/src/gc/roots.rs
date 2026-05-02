use std::sync::Mutex;

use crate::alloc::GcObjectHeader;
#[cfg(test)]
use crate::alloc::ruyi_dealloc;

/// Set of GC roots split into stack and global roots.
///
/// Stack roots correspond to live local variables in active call frames.
/// Global roots correspond to module-level static variables.
pub struct RootSet {
    stack_roots: Mutex<Vec<*mut GcObjectHeader>>,
    global_roots: Mutex<Vec<*mut GcObjectHeader>>,
}

impl RootSet {
    pub fn new() -> Self {
        Self {
            stack_roots: Mutex::new(Vec::new()),
            global_roots: Mutex::new(Vec::new()),
        }
    }

    /// Register a stack-local root.
    ///
    /// # Safety
    /// `ptr` must point to the payload of a valid GC object.
    pub unsafe fn add_stack_root(&self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }
        let header = GcObjectHeader::from_payload(ptr);
        (*header).set_pinned();
        self.stack_roots.lock().unwrap().push(header);
    }

    /// Unregister a stack-local root.
    ///
    /// # Safety
    /// `ptr` must have been previously passed to `add_stack_root`.
    pub unsafe fn remove_stack_root(&self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }
        let header = GcObjectHeader::from_payload(ptr);
        (*header).clear_pinned();
        let mut roots = self.stack_roots.lock().unwrap();
        if let Some(pos) = roots.iter().position(|&h| h == header) {
            roots.swap_remove(pos);
        }
    }

    /// Register a global root.
    ///
    /// # Safety
    /// `ptr` must point to the payload of a valid GC object.
    pub unsafe fn add_global_root(&self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }
        let header = GcObjectHeader::from_payload(ptr);
        (*header).set_pinned();
        self.global_roots.lock().unwrap().push(header);
    }

    /// Unregister a global root.
    ///
    /// # Safety
    /// `ptr` must have been previously passed to `add_global_root`.
    pub unsafe fn remove_global_root(&self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }
        let header = GcObjectHeader::from_payload(ptr);
        (*header).clear_pinned();
        let mut roots = self.global_roots.lock().unwrap();
        if let Some(pos) = roots.iter().position(|&h| h == header) {
            roots.swap_remove(pos);
        }
    }

    /// Return a combined list of all roots (stack + global).
    pub fn all_roots(&self) -> Vec<*mut GcObjectHeader> {
        let mut result = self.stack_roots.lock().unwrap().clone();
        result.extend(self.global_roots.lock().unwrap().iter().copied());
        result
    }

    pub fn stack_roots(&self) -> Vec<*mut GcObjectHeader> {
        self.stack_roots.lock().unwrap().clone()
    }

    pub fn global_roots(&self) -> Vec<*mut GcObjectHeader> {
        self.global_roots.lock().unwrap().clone()
    }

    pub fn stack_count(&self) -> usize {
        self.stack_roots.lock().unwrap().len()
    }

    pub fn global_count(&self) -> usize {
        self.global_roots.lock().unwrap().len()
    }
}

impl Default for RootSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc::{MemoryStrategy, TypeInfo, ruyi_alloc};

    #[test]
    fn test_add_remove_stack_root() {
        static mut TYPE_INFO: TypeInfo = TypeInfo {
            type_id: 1,
            type_name: "test",
            destructor: None,
            trace_fn: None,
        };

        let roots = RootSet::new();
        unsafe {
            let ptr = ruyi_alloc(8, &raw mut TYPE_INFO, MemoryStrategy::GC);
            roots.add_stack_root(ptr);
            assert_eq!(roots.stack_count(), 1);

            roots.remove_stack_root(ptr);
            assert_eq!(roots.stack_count(), 0);

            ruyi_dealloc(ptr);
        }
    }

    #[test]
    fn test_all_roots() {
        static mut TYPE_INFO: TypeInfo = TypeInfo {
            type_id: 2,
            type_name: "test",
            destructor: None,
            trace_fn: None,
        };

        let roots = RootSet::new();
        unsafe {
            let a = ruyi_alloc(8, &raw mut TYPE_INFO, MemoryStrategy::GC);
            let b = ruyi_alloc(8, &raw mut TYPE_INFO, MemoryStrategy::GC);

            roots.add_stack_root(a);
            roots.add_global_root(b);

            let all = roots.all_roots();
            assert_eq!(all.len(), 2);

            ruyi_dealloc(a);
            ruyi_dealloc(b);
        }
    }
}
