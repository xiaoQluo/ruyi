//! Async runtime C exports for the green-thread scheduler.
//!
//! Provides `extern "C"` functions that wrap the async scheduler so that
//! the compiler frontend (LLVM code generator) can emit calls to runtime
//! async routines.
//!
//! @author Ruyi Team
//! @date 2026-05-03

use std::sync::Mutex;
use once_cell::sync::Lazy;

use crate::async_runtime::{Poll, RuyiFuture, Scheduler, TaskId, Waker};

// ── Global singleton ─────────────────────────────────────────

/// Global scheduler instance (baseline: single worker thread).
///
/// Uses the same `Lazy<Mutex<…>>` pattern as `gc_exports.rs`.
static GLOBAL_SCHEDULER: Lazy<Mutex<Scheduler>> =
    Lazy::new(|| Mutex::new(Scheduler::new(1)));

/// Wrapper that turns an opaque C future pointer into a `RuyiFuture`.
///
/// The codegen (T05) will produce future state machines as raw
/// heap-allocated structs. Until the vtable convention is defined,
/// this wrapper provides a minimal baseline `RuyiFuture` implementation.
struct CFuture {
    ptr: *mut u8,
}

// Safety: `*mut u8` is `Send`, and the pointer is treated as opaque.
unsafe impl Send for CFuture {}

impl RuyiFuture for CFuture {
    type Output = ();

    fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
        // Baseline: opaque future cannot be polled without a codegen-
        // provided vtable (T05). Return Ready so the task completes
        // immediately rather than hanging the scheduler.
        let _ = self.ptr;
        Poll::Ready(())
    }
}

// ── C exports ────────────────────────────────────────────────

/// Poll a future once.
///
/// # Safety
/// `future_ptr` must be a valid pointer to a codegen-generated future.
/// `waker_ptr` must be a valid pointer to a runtime `Waker`.
///
/// Returns `0` for Pending, `1` for Ready.
#[no_mangle]
pub extern "C" fn ruyi_async_poll(_future_ptr: *mut u8, _waker_ptr: *mut u8) -> i32 {
    // Baseline placeholder: opaque future cannot be polled without a
    // codegen-provided vtable (T05). Return Ready (1).
    1
}

/// Spawn a future onto the global scheduler.
///
/// # Safety
/// `future_ptr` must be a valid pointer to a heap-allocated future.
///
/// Returns an opaque task handle (cast from the internal `TaskId`).
#[no_mangle]
pub extern "C" fn ruyi_spawn(future_ptr: *mut u8) -> *mut u8 {
    let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
    let task_id = scheduler.spawn(CFuture { ptr: future_ptr });
    task_id.0 as *mut u8
}

/// Wake a previously spawned task.
///
/// # Safety
/// `task_ptr` must be a task handle returned by `ruyi_spawn`.
#[no_mangle]
pub extern "C" fn ruyi_wake_task(task_ptr: *mut u8) {
    let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
    let task_id = TaskId(task_ptr as usize);
    let waker = scheduler.test_waker(task_id);
    waker.wake();
}

/// Run the global scheduler until all tasks have completed.
#[no_mangle]
pub extern "C" fn ruyi_run_scheduler() {
    let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
    scheduler.block_on_all();
}
