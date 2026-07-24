//! Async runtime C exports for the green-thread scheduler.
//!
//! Provides `extern "C"` functions that wrap the async scheduler so that
//! the compiler frontend (LLVM code generator) can emit calls to runtime
//! async routines.
//!
//! @author Ruyi Team
//! @date 2026-05-03

use crate::async_runtime::{Poll, RuyiFuture, TaskId, Waker, GLOBAL_SCHEDULER};

/// Wrapper that turns an opaque C future pointer into a `RuyiFuture`.
///
/// The codegen (T05) will produce future state machines as raw
/// heap-allocated structs. Until the vtable convention is defined,
/// this wrapper provides a minimal baseline `RuyiFuture` implementation.
type PollFn = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;

struct CFuture {
    ptr: *mut u8,
}

unsafe impl Send for CFuture {}

impl RuyiFuture for CFuture {
    type Output = ();

    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        let poll_fn_ptr = unsafe {
            let ptr_val = std::ptr::read::<*mut u8>(self.ptr as *const *mut u8);
            std::mem::transmute::<*mut u8, PollFn>(ptr_val)
        };
        let waker_ptr = waker as *const Waker as *mut u8;
        let result = unsafe { poll_fn_ptr(self.ptr, waker_ptr) };
        if result == 1 {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
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
pub unsafe extern "C" fn ruyi_async_poll(future_ptr: *mut u8, waker_ptr: *mut u8) -> i32 {
    let poll_fn_ptr = unsafe {
        let ptr_val = std::ptr::read::<*mut u8>(future_ptr as *const *mut u8);
        std::mem::transmute::<*mut u8, PollFn>(ptr_val)
    };
    unsafe { poll_fn_ptr(future_ptr, waker_ptr) }
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
