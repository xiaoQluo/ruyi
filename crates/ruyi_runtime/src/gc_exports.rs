//! GC C exports for the generational collector.
//!
//! Provides `extern "C"` functions that wrap the generational garbage
//! collector so that the compiler frontend (LLVM code generator) can
//! emit calls to runtime GC routines.
//!
//! ## Thread Safety
//!
//! Each OS thread owns its own `GenerationalCollector` via `thread_local!`.
//! GC objects allocated in one thread MUST NOT be accessed from another
//! thread — doing so will cause use-after-free or data corruption since
//! the cross-thread collector has no knowledge of foreign objects.
//!
//! To share data across threads, use `Arc<T>`, `Mutex<T>`, `Channel<T>`,
//! or `Atomic<int>` — all of which are thread-safe and do not involve
//! GC-managed memory.
//!
//! The `CURRENT_COLLECTOR` is auto-initialized on first access in each
//! thread. New threads spawned via `__thread_spawn` automatically receive
//! their own collector instance.

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
///
/// Before sweeping, this consults the async GC root registry
/// (`async_gc_roots::snapshot`) so that objects reachable only from a
/// suspended async task are retained across the collection. The scan
/// uses the same word-wise traversal as
/// `async_runtime::register_async_roots` to find GC pointers stored in
/// each registered future.
#[no_mangle]
pub extern "C" fn ruyi_gc_collect() {
    let task_ids = crate::async_gc_roots::snapshot();
    if let Ok(scheduler) = crate::async_runtime::GLOBAL_SCHEDULER.try_lock() {
        let tasks = scheduler.inner.tasks.lock().unwrap();
        CURRENT_COLLECTOR.with(|collector| {
            let collector = collector.borrow_mut();
            let allowed: std::collections::HashSet<usize> = task_ids.into_iter().collect();
            for (task_id, task) in tasks.iter() {
                if !allowed.contains(&task_id.0) {
                    continue;
                }
                let future_ref: &(dyn crate::async_runtime::RuyiFuture<Output = ()> + Send) =
                    &*task.future;
                let data_ptr = future_ref
                    as *const (dyn crate::async_runtime::RuyiFuture<Output = ()> + Send)
                    as *const u8;
                let size = std::mem::size_of_val(future_ref);
                if data_ptr.is_null() || size == 0 {
                    continue;
                }
                let step = std::mem::size_of::<usize>();
                let mut offset = 0;
                while offset + step <= size {
                    let word =
                        unsafe { std::ptr::read_unaligned(data_ptr.add(offset) as *const usize) };
                    let candidate = word as *mut u8;
                    if !candidate.is_null() && collector.is_valid_payload(candidate) {
                        unsafe {
                            collector.add_root(candidate);
                        }
                    }
                    offset += step;
                }
            }
            collector.collect_full();
        });
    } else {
        CURRENT_COLLECTOR.with(|collector| {
            let collector = collector.borrow_mut();
            collector.collect_full();
        })
    }
}

/// Add a stack root to the GC.
///
/// # Safety
/// `ptr` must point to the payload of a valid GC object.
#[no_mangle]
pub unsafe extern "C" fn ruyi_gc_add_root(ptr: *mut u8) {
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
pub unsafe extern "C" fn ruyi_gc_remove_root(ptr: *mut u8) {
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
pub unsafe extern "C" fn ruyi_gc_write_barrier(parent: *mut u8, field: *mut u8) {
    CURRENT_COLLECTOR.with(|collector| {
        let collector = collector.borrow_mut();
        unsafe {
            let header = GcObjectHeader::from_payload(parent);
            collector.write_barrier(header, std::ptr::null_mut(), field);
        }
    });
}
