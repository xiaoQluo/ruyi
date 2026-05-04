//! GC C exports for the generational collector.
//!
//! Provides `extern "C"` functions that wrap the generational garbage
//! collector so that the compiler frontend (LLVM code generator) can
//! emit calls to runtime GC routines.

use std::cell::RefCell;

use crate::alloc::{GcObjectHeader, TypeInfo};
use crate::gc::generational::GenerationalCollector;

thread_local! {
    static CURRENT_COLLECTOR: RefCell<GenerationalCollector> =
        RefCell::new(GenerationalCollector::new());
}

static mut DUMMY_TYPE_INFO: TypeInfo = TypeInfo {
    type_id: 0,
    type_name: "unknown",
    destructor: None,
    trace_fn: None,
};

/// Allocate a GC-managed object in the young generation.
///
/// # Safety
/// `size` must be non-negative.
#[no_mangle]
pub extern "C" fn ruyi_gc_alloc(size: i64) -> *mut u8 {
    let type_info = &raw mut DUMMY_TYPE_INFO;
    CURRENT_COLLECTOR.with(|collector| {
        let collector = collector.borrow_mut();
        unsafe { collector.allocate(size as usize, type_info) }
    })
}

/// Trigger a full GC collection (both young and old generations).
#[no_mangle]
pub extern "C" fn ruyi_gc_collect() {
    CURRENT_COLLECTOR.with(|collector| {
        let collector = collector.borrow_mut();
        collector.collect_full();
    })
}

/// Add a stack root to the GC.
///
/// # Safety
/// `ptr` must point to the payload of a valid GC object.
#[no_mangle]
pub extern "C" fn ruyi_gc_add_root(ptr: *mut u8) {
    CURRENT_COLLECTOR.with(|collector| {
        let collector = collector.borrow_mut();
        unsafe {
            collector.add_root(ptr);
        }
    });
}

/// Remove a previously registered stack root.
///
/// # Safety
/// `ptr` must have been previously passed to `ruyi_gc_add_root`.
#[no_mangle]
pub extern "C" fn ruyi_gc_remove_root(ptr: *mut u8) {
    CURRENT_COLLECTOR.with(|collector| {
        let collector = collector.borrow_mut();
        unsafe {
            collector.remove_root(ptr);
        }
    });
}

/// Record a cross-generational reference via the write barrier.
///
/// # Safety
/// `parent` must point to the payload of a valid GC object.
/// `field` may be null or a valid GC payload pointer.
#[no_mangle]
pub extern "C" fn ruyi_gc_write_barrier(parent: *mut u8, field: *mut u8) {
    CURRENT_COLLECTOR.with(|collector| {
        let collector = collector.borrow_mut();
        unsafe {
            let header = GcObjectHeader::from_payload(parent);
            collector.write_barrier(header, std::ptr::null_mut(), field);
        }
    });
}
