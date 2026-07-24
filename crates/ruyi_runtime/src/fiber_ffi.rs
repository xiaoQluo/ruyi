//! Fiber (纎程) — lightweight user-level concurrency primitives.
//!
//! Fibers are cooperative, lightweight tasks that run on OS threads.
//! In v0.5.10 each fiber is backed by an OS-thread from a fixed-size pool;
//! future versions will migrate to the async scheduler for true green threads.
//!
//! The API surface is designed for eventual migration:
//! - `__fiber_spawn()` / `__fiber_join()` mirror async task lifecycle
//! - `__fiber_yield()` / `__fiber_sleep()` prepare for cooperative scheduling
//!
//! # Memory model
//! Each fiber is bound to its creator thread's GC heap (per the memory model).
//!
//! @author Ruyi Team
//! @date 2026-07-25

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{mpsc, Mutex};
use std::thread;

// ── Fiber registry ────────────────────────────────────────────

type FiberFn = extern "C" fn(usize);

/// A spawned fiber: the thread handle (if joined) and a completion signal.
struct Fiber {
    handle: Option<thread::JoinHandle<()>>,
    receiver: mpsc::Receiver<()>,
}

static FIBERS: Mutex<Option<HashMap<i64, Fiber>>> = Mutex::new(None);
static NEXT_FIBER_ID: AtomicI64 = AtomicI64::new(0);

fn ensure_registry() {
    let mut guard = FIBERS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
}

fn next_id() -> i64 {
    NEXT_FIBER_ID.fetch_add(1, Ordering::SeqCst) + 1
}

// ── FFI exports ─────────────────────────────────────────────────

/// Spawn a new fiber.
///
/// The entry function `entry(arg)` runs on an OS thread from the pool.
/// Returns a positive fiber handle on success, -1 on failure.
///
/// # Safety
/// `entry` must be a valid function pointer.
#[no_mangle]
pub unsafe extern "C" fn __fiber_spawn(entry: *mut i8, arg: *mut i8) -> i64 {
    if entry.is_null() {
        return -1;
    }
    let entry_fn: FiberFn = unsafe { std::mem::transmute(entry) };
    let arg_val = arg as usize;
    let (tx, rx) = mpsc::channel();

    match thread::Builder::new()
        .stack_size(256 * 1024) // 256 KiB — smaller than thread (2 MiB)
        .spawn(move || {
            entry_fn(arg_val);
            let _ = tx.send(()); // signal completion
        }) {
        Ok(handle) => {
            ensure_registry();
            let id = next_id();
            let mut registry = FIBERS.lock().unwrap();
            registry.as_mut().unwrap().insert(
                id,
                Fiber {
                    handle: Some(handle),
                    receiver: rx,
                },
            );
            id
        }
        Err(_) => -1,
    }
}

/// Wait for a fiber to complete (join).
///
/// Blocks the calling thread until the fiber finishes.
/// Returns 0 on success, -1 if the handle is invalid.
#[no_mangle]
pub extern "C" fn __fiber_join(handle: i64) -> i64 {
    ensure_registry();
    let fiber = {
        let mut registry = FIBERS.lock().unwrap();
        registry.as_mut().unwrap().remove(&handle)
    };
    match fiber {
        Some(f) => {
            // Wait for the completion signal, then join the thread.
            let _ = f.receiver.recv();
            if let Some(h) = f.handle {
                let _ = h.join();
            }
            0
        }
        None => -1,
    }
}

/// Check whether a fiber has finished, without consuming the handle.
///
/// Returns 1 if finished, 0 if still running or handle invalid.
#[no_mangle]
pub extern "C" fn __fiber_is_finished(handle: i64) -> i64 {
    ensure_registry();
    let registry = FIBERS.lock().unwrap();
    match registry.as_ref().unwrap().get(&handle) {
        Some(f) => match f.receiver.try_recv() {
            Ok(_) => 1,
            Err(mpsc::TryRecvError::Disconnected) => 1,
            Err(mpsc::TryRecvError::Empty) => 0,
        },
        None => 0,
    }
}

/// Detach a fiber — it runs independently.
///
/// The fiber's resources are reclaimed automatically when it finishes.
/// Returns 0 on success, -1 if the handle is invalid.
#[no_mangle]
pub extern "C" fn __fiber_detach(handle: i64) -> i64 {
    ensure_registry();
    let fiber = {
        let mut registry = FIBERS.lock().unwrap();
        registry.as_mut().unwrap().remove(&handle)
    };
    match fiber {
        Some(f) => {
            // Take the handle so it gets dropped (but NOT joined).
            // The thread will continue running independently.
            drop(f);
            0
        }
        None => -1,
    }
}

/// Get the current fiber ID.
///
/// Returns a platform-dependent integer for display/logging purposes.
/// In v0.5.10 this returns the OS thread ID; future versions will
/// return the scheduler task ID.
#[no_mangle]
pub extern "C" fn __fiber_id() -> i64 {
    let id = thread::current().id();
    format!("{:?}", id)
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

/// Sleep the current fiber for `ms` milliseconds.
///
/// In v0.5.10 this is `thread::sleep`; in future versions it will
/// cooperatively yield to the scheduler.
#[no_mangle]
pub extern "C" fn __fiber_sleep(ms: i64) {
    if ms <= 0 {
        return;
    }
    thread::sleep(std::time::Duration::from_millis(ms as u64));
}

/// Yield the current fiber — hint to the scheduler that this fiber
/// is willing to let others run.
///
/// In v0.5.10 this is `thread::yield_now`.
#[no_mangle]
pub extern "C" fn __fiber_yield() {
    thread::yield_now();
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    extern "C" fn simple_entry(arg: usize) {
        let val = arg as i64;
        // Just double the argument and store it nowhere observable —
        // this is a smoke test.
        let _ = val * 2;
    }

    extern "C" fn counting_entry(flag: usize) {
        let done = unsafe { &*(flag as *const AtomicBool) };
        done.store(true, Ordering::SeqCst);
    }

    #[test]
    fn test_fiber_spawn_and_join() {
        let handle = unsafe { __fiber_spawn(simple_entry as *mut i8, 42 as *mut i8) };
        assert!(handle > 0, "fiber spawn should return positive handle");
        let result = __fiber_join(handle);
        assert_eq!(result, 0, "fiber join should succeed");
    }

    #[test]
    fn test_fiber_join_twice() {
        let handle = unsafe { __fiber_spawn(simple_entry as *mut i8, std::ptr::null_mut()) };
        assert!(handle > 0);
        assert_eq!(__fiber_join(handle), 0);
        // Second join should fail (fiber already consumed).
        assert_eq!(__fiber_join(handle), -1);
    }

    #[test]
    fn test_fiber_is_finished() {
        let done = Arc::new(AtomicBool::new(false));
        let flag_ptr = Arc::as_ptr(&done) as usize;

        let handle = unsafe { __fiber_spawn(counting_entry as *mut i8, flag_ptr as *mut i8) };
        assert!(handle > 0);

        // Wait for the fiber to finish.
        let start = std::time::Instant::now();
        loop {
            if __fiber_is_finished(handle) == 1 {
                break;
            }
            if start.elapsed() > std::time::Duration::from_secs(5) {
                panic!("fiber did not finish within 5 seconds");
            }
            thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(done.load(Ordering::SeqCst));
        assert_eq!(__fiber_join(handle), 0);
    }

    #[test]
    fn test_fiber_detach() {
        let done = Arc::new(AtomicBool::new(false));
        let flag_ptr = Arc::as_ptr(&done) as usize;

        let handle = unsafe { __fiber_spawn(counting_entry as *mut i8, flag_ptr as *mut i8) };
        assert!(handle > 0);

        assert_eq!(__fiber_detach(handle), 0);
        // Second detach should fail.
        assert_eq!(__fiber_detach(handle), -1);

        // Wait for the detached fiber to finish.
        let start = std::time::Instant::now();
        while !done.load(Ordering::SeqCst) {
            if start.elapsed() > std::time::Duration::from_secs(5) {
                panic!("detached fiber did not finish within 5 seconds");
            }
            thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    #[test]
    fn test_fiber_id() {
        let id = __fiber_id();
        assert!(id >= 0, "fiber ID should be non-negative");
    }

    #[test]
    fn test_fiber_sleep() {
        // Smoke test: sleep for 1ms should not panic.
        __fiber_sleep(1);
    }

    #[test]
    fn test_fiber_yield() {
        // Smoke test: yield should not panic.
        __fiber_yield();
    }

    #[test]
    fn test_fiber_spawn_null_entry() {
        let handle = unsafe { __fiber_spawn(std::ptr::null_mut(), std::ptr::null_mut()) };
        assert_eq!(handle, -1, "null entry should return -1");
    }
}
