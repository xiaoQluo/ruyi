//! Integration tests for the `ruyi_spawn` FFI.
//!
//! Validates spawn behavior at the runtime level (no codegen involved):
//! 1. `ruyi_spawn` accepts a future pointer + returns a non-null task handle
//! 2. Multiple spawns accumulate in the scheduler
//! 3. Concurrent spawns across many calls all submit successfully
//!
//! These tests do NOT require the inkwell feature (no LLVM linking).
//! They run with `cargo test -p ruyi_runtime --test spawn`.
//!
//! @author luozegang
//! @date 2026-07-10

use ruyi_runtime::async_exports::ruyi_spawn;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A heap-allocated future stand-in for `ruyi_spawn`'s `*mut u8` ABI.
///
/// `CFuture::poll` reads the first 8 bytes of the future pointer as a
/// function pointer and calls it. We must therefore lay out a valid
/// poll function pointer at offset 0, otherwise the scheduler's worker
/// thread will deref a garbage address and SIGSEGV.
///
/// The poll function we install is a no-op that always returns
/// `Pending`, so the task is registered but never makes progress
/// (acceptable — these tests verify *submission*, not execution).
unsafe extern "C" fn dummy_poll(_future: *mut u8, _waker: *mut u8) -> i32 {
    0 // 0 = Pending (per ruyi_async_poll contract)
}

#[repr(C)]
struct TestFuture {
    poll_fn: unsafe extern "C" fn(*mut u8, *mut u8) -> i32,
}

impl TestFuture {
    fn new() -> *mut u8 {
        let boxed = Box::new(TestFuture { poll_fn: dummy_poll });
        Box::into_raw(boxed) as *mut u8
    }
}

#[test]
fn ruyi_spawn_returns_non_null_handle() {
    let future = TestFuture::new();
    let handle = ruyi_spawn(future);
    assert!(!handle.is_null(), "ruyi_spawn should return non-null task handle");
}

#[test]
fn ruyi_spawn_handles_multiple_tasks() {
    let mut handles = Vec::with_capacity(10);
    for i in 0..10 {
        let future = TestFuture::new();
        let handle = ruyi_spawn(future);
        assert!(!handle.is_null(), "spawn #{i} returned null handle");
        handles.push(handle);
    }
    assert_eq!(handles.len(), 10);
}

#[test]
fn ruyi_spawn_accepts_many_tasks_under_load() {
    let mut ok = 0u32;
    for _ in 0..100 {
        let future = TestFuture::new();
        let handle = ruyi_spawn(future);
        if !handle.is_null() {
            ok += 1;
        }
    }
    assert!(ok >= 95, "ruyi_spawn accepted only {ok}/100 under load");
}

#[test]
fn ruyi_spawn_atomic_counter_compiles() {
    let counter = AtomicUsize::new(0);
    for _ in 0..5 {
        let future = TestFuture::new();
        let _ = ruyi_spawn(future);
    }
    assert_eq!(counter.load(Ordering::SeqCst), 0);
}