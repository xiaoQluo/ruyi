#![allow(clippy::not_unsafe_ptr_arg_deref)]

/**
 * Thread FFI — OS thread spawning and management.
 *
 * Provides native OS thread creation, joining, and identification.
 * Each spawned thread automatically receives its own per-thread
 * `GenerationalCollector` instance via the `thread_local!` in
 * `gc_exports.rs`, ensuring GC isolation between threads.
 *
 * Thread entry points are raw function pointers (`extern "C" fn(*mut i8)`)
 * accepting an opaque `*mut i8` argument — the caller is responsible
 * for packing/unpacking arguments.
 *
 * Thread handles are `i64` IDs tracked in a global registry.
 *
 * @author Ruyi Team
 * @date 2026-07-25
 */
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

// ── Global thread registry ──────────────────────────────────────

type ThreadFn = extern "C" fn(usize);
type ThreadEntry = Option<JoinHandle<()>>;

static THREADS: Mutex<Option<HashMap<i64, ThreadEntry>>> = Mutex::new(None);
static NEXT_HANDLE: AtomicI64 = AtomicI64::new(0);

fn ensure_registry() {
    let mut guard = THREADS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
}

fn next_handle() -> i64 {
    NEXT_HANDLE.fetch_add(1, Ordering::SeqCst) + 1
}

fn register_thread(handle: JoinHandle<()>) -> i64 {
    ensure_registry();
    let id = next_handle();
    let mut registry = THREADS.lock().unwrap();
    registry.as_mut().unwrap().insert(id, Some(handle));
    id
}

fn take_thread(id: i64) -> Option<JoinHandle<()>> {
    ensure_registry();
    let mut registry = THREADS.lock().unwrap();
    registry.as_mut().unwrap().remove(&id).flatten()
}

// ── FFI exports ─────────────────────────────────────────────────

/// Spawn a new OS thread executing `entry(arg)`.
///
/// The thread receives its own `GenerationalCollector` automatically
/// via `gc_exports::CURRENT_COLLECTOR` (thread-local).
///
/// Returns a positive handle on success, or -1 on failure.
/// The handle must be joined via `__thread_join` or detached via
/// `__thread_detach`.
#[no_mangle]
pub extern "C" fn __thread_spawn(entry: *mut i8, arg: *mut i8) -> i64 {
    if entry.is_null() {
        return -1;
    }
    let entry_fn: ThreadFn = unsafe { std::mem::transmute(entry) };
    let arg_val = arg as usize;

    match thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            entry_fn(arg_val);
        }) {
        Ok(handle) => register_thread(handle),
        Err(_) => -1,
    }
}

/// Join a spawned thread, blocking until it completes.
///
/// Returns 0 on success, -1 if the handle is invalid or already joined.
#[no_mangle]
pub extern "C" fn __thread_join(handle: i64) -> i64 {
    match take_thread(handle) {
        Some(join_handle) => match join_handle.join() {
            Ok(()) => 0,
            Err(_) => -1,
        },
        None => -1,
    }
}

/// Detach a spawned thread — it will run independently.
///
/// The thread's resources are reclaimed automatically when it finishes.
/// Returns 0 on success, -1 if the handle is invalid.
#[no_mangle]
pub extern "C" fn __thread_detach(handle: i64) -> i64 {
    if take_thread(handle).is_some() {
        0
    } else {
        -1
    }
}

/// Get the current OS thread ID as a platform-dependent integer.
/// For display/logging purposes only.
#[no_mangle]
pub extern "C" fn __thread_id() -> i64 {
    // Use the thread name hash as a stable-ish ID within the process.
    let id = thread::current().id();
    // ThreadId's as_u64 is nightly-only, use debug format
    format!("{:?}", id)
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

/// Get the number of available CPU cores (logical).
#[no_mangle]
pub extern "C" fn __thread_cpu_count() -> i64 {
    thread::available_parallelism()
        .map(|n| n.get() as i64)
        .unwrap_or(1)
}

/// Sleep the current thread for `ms` milliseconds.
#[no_mangle]
pub extern "C" fn __thread_sleep(ms: i64) {
    if ms <= 0 {
        return;
    }
    thread::sleep(std::time::Duration::from_millis(ms as u64));
}

#[cfg(test)]
mod tests {
    use super::*;

    extern "C" fn dummy_entry(_arg: usize) {}

    #[test]
    fn test_spawn_and_join() {
        let handle = __thread_spawn(dummy_entry as *mut i8, std::ptr::null_mut());
        assert!(handle > 0, "spawn should return positive handle");

        let result = __thread_join(handle);
        assert_eq!(result, 0, "join should succeed");
    }

    #[test]
    fn test_detach() {
        let handle = __thread_spawn(dummy_entry as *mut i8, std::ptr::null_mut());
        assert!(handle > 0);

        let result = __thread_detach(handle);
        assert_eq!(result, 0, "detach should succeed");

        // Joining after detach should fail
        let result = __thread_join(handle);
        assert_eq!(result, -1, "join after detach should fail");
    }

    #[test]
    fn test_invalid_handle() {
        assert_eq!(__thread_join(99999), -1);
        assert_eq!(__thread_detach(99999), -1);
    }

    #[test]
    fn test_thread_id() {
        let id = __thread_id();
        assert!(id >= 0, "thread ID should be non-negative");
    }

    #[test]
    fn test_cpu_count() {
        let count = __thread_cpu_count();
        assert!(count >= 1, "CPU count should be at least 1");
    }

    #[test]
    fn test_sleep() {
        // Just verify it doesn't panic
        __thread_sleep(1);
        __thread_sleep(0);
        __thread_sleep(-1);
    }
}
