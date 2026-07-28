#![allow(clippy::not_unsafe_ptr_arg_deref)]
/**
 * Semaphore FFI — counting semaphore for resource limiting.
 *
 * Built on `std::sync::Mutex<usize>` + `std::sync::Condvar`.
 * Supports blocking acquire and non-blocking try_acquire.
 *
 * Typical use: connection pool limiting, rate control, bounded
 * producer-consumer scenarios.
 *
 * @author Ruyi Team
 * @date 2026-07-25
 */
use std::sync::{Condvar, Mutex};

/// Internal representation of a counting semaphore.
pub struct Semaphore {
    permits: Mutex<usize>,
    condvar: Condvar,
}

/// Create a new semaphore with `n` initial permits.
#[no_mangle]
pub extern "C" fn __semaphore_new(n: i32) -> *mut Semaphore {
    assert!(n >= 0, "Semaphore permits must be non-negative");
    Box::into_raw(Box::new(Semaphore {
        permits: Mutex::new(n as usize),
        condvar: Condvar::new(),
    }))
}

/// Acquire a permit, blocking until one is available.
#[no_mangle]
pub extern "C" fn __semaphore_acquire(ptr: *mut Semaphore) {
    if ptr.is_null() {
        return;
    }
    let s = unsafe { &*ptr };
    let mut permits = s.permits.lock().unwrap();
    while *permits == 0 {
        permits = s.condvar.wait(permits).unwrap();
    }
    *permits -= 1;
}

/// Try to acquire a permit without blocking.
/// Returns 1 on success, 0 if no permits are available.
#[no_mangle]
pub extern "C" fn __semaphore_try_acquire(ptr: *mut Semaphore) -> i32 {
    if ptr.is_null() {
        return 0;
    }
    let s = unsafe { &*ptr };
    let mut permits = s.permits.lock().unwrap();
    if *permits > 0 {
        *permits -= 1;
        1
    } else {
        0
    }
}

/// Release a permit back to the semaphore.
/// Wakes one waiting acquirer if any.
#[no_mangle]
pub extern "C" fn __semaphore_release(ptr: *mut Semaphore) {
    if ptr.is_null() {
        return;
    }
    let s = unsafe { &*ptr };
    let mut permits = s.permits.lock().unwrap();
    *permits += 1;
    s.condvar.notify_one();
}

/// Return the current number of available permits (snapshot — not atomic).
#[no_mangle]
pub extern "C" fn __semaphore_available(ptr: *mut Semaphore) -> i32 {
    if ptr.is_null() {
        return 0;
    }
    let s = unsafe { &*ptr };
    let permits = s.permits.lock().unwrap();
    *permits as i32
}

/// Deallocate the semaphore.  Must not be in use when called.
#[no_mangle]
pub extern "C" fn __semaphore_free(ptr: *mut Semaphore) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_semaphore_basic() {
        let s = __semaphore_new(2);
        __semaphore_acquire(s);
        assert_eq!(__semaphore_available(s), 1);
        __semaphore_release(s);
        assert_eq!(__semaphore_available(s), 2);
        __semaphore_free(s);
    }

    #[test]
    fn test_semaphore_try_acquire() {
        let s = __semaphore_new(1);
        assert_eq!(__semaphore_try_acquire(s), 1);
        assert_eq!(__semaphore_try_acquire(s), 0);
        __semaphore_release(s);
        assert_eq!(__semaphore_try_acquire(s), 1);
        __semaphore_free(s);
    }

    #[test]
    fn test_semaphore_blocking() {
        let s = __semaphore_new(0);
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        let s_raw = s as usize;

        let h = thread::spawn(move || {
            c.store(1, Ordering::SeqCst);
            __semaphore_acquire(s_raw as *mut Semaphore);
            c.store(2, Ordering::SeqCst);
        });

        // Give thread time to start and block
        thread::sleep(std::time::Duration::from_millis(100));
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        __semaphore_release(s);
        h.join().unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 2);
        __semaphore_free(s);
    }
}
