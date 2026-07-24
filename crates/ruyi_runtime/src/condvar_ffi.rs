#![allow(clippy::not_unsafe_ptr_arg_deref)]
/**
 * Condvar FFI — condition variable for thread coordination.
 *
 * Wraps `std::sync::Condvar`.  Paired with `__mutex_*` functions:
 * the caller must hold the mutex (have a valid guard) before calling
 * `wait()`.  `wait()` atomically unlocks the mutex, blocks until
 * notified, re-locks the mutex, and returns a new guard.
 *
 * The old guard pointer is consumed (dropped) by `wait()`.  The
 * caller MUST NOT use the old guard after calling `wait()`.
 *
 * @author Ruyi Team
 * @date 2026-07-25
 */
use std::sync::{Condvar, Mutex, MutexGuard};

/// Create a new condition variable.
#[no_mangle]
pub extern "C" fn __condvar_new() -> *mut Condvar {
    Box::into_raw(Box::new(Condvar::new()))
}

/// Wait on the condition variable.
///
/// `mutex_ptr` is the Mutex pointer (the one from `__mutex_new`),
/// NOT the guard.  This function internally acquires the lock,
/// waits, and returns a new guard.  The caller must call
/// `__mutex_unlock(new_guard)` to release after inspecting the condition.
///
/// Returns a new mutex guard pointer.
///
/// # Safety
/// The mutex must be unlocked when this is called.  After this returns,
/// the mutex is locked (the returned guard must be unlocked).
#[no_mangle]
pub extern "C" fn __condvar_wait(
    cv_ptr: *mut Condvar,
    mutex_ptr: *mut Mutex<()>,
) -> *mut MutexGuard<'static, ()> {
    if cv_ptr.is_null() || mutex_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let cv = unsafe { &*cv_ptr };
    let m = unsafe { &*mutex_ptr };
    let guard = m.lock().unwrap();
    let new_guard = cv.wait(guard).unwrap();
    Box::into_raw(Box::new(new_guard))
}

/// Wake one thread waiting on this condition variable.
#[no_mangle]
pub extern "C" fn __condvar_notify_one(ptr: *mut Condvar) {
    if ptr.is_null() {
        return;
    }
    let cv = unsafe { &*ptr };
    cv.notify_one();
}

/// Wake all threads waiting on this condition variable.
#[no_mangle]
pub extern "C" fn __condvar_notify_all(ptr: *mut Condvar) {
    if ptr.is_null() {
        return;
    }
    let cv = unsafe { &*ptr };
    cv.notify_all();
}

/// Deallocate the condition variable.  Must not be in use.
#[no_mangle]
pub extern "C" fn __condvar_free(ptr: *mut Condvar) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mutex_ffi::{__mutex_free, __mutex_lock, __mutex_new, __mutex_unlock};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_condvar_notify_one() {
        let cv = __condvar_new();
        let m = __mutex_new();
        let ready = Arc::new(AtomicBool::new(false));
        let r = ready.clone();
        let cv_raw = cv as usize;
        let m_raw = m as usize;

        let h = thread::spawn(move || {
            let cv_ptr = cv_raw as *mut Condvar;
            let m_ptr = m_raw as *mut Mutex<()>;
            let guard = __condvar_wait(cv_ptr, m_ptr);
            assert!(r.load(Ordering::SeqCst));
            __mutex_unlock(guard as *mut i8);
        });

        thread::sleep(std::time::Duration::from_millis(50));
        ready.store(true, Ordering::SeqCst);
        __condvar_notify_one(cv);
        h.join().unwrap();

        __mutex_free(m);
        __condvar_free(cv);
    }

    #[test]
    fn test_condvar_notify_all() {
        let cv = __condvar_new();
        let m = __mutex_new();
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cv_raw = cv as usize;
        let m_raw = m as usize;

        let mut handles = vec![];
        for _ in 0..4 {
            let c = counter.clone();
            let h = thread::spawn(move || {
                let cv_ptr = cv_raw as *mut Condvar;
                let m_ptr = m_raw as *mut Mutex<()>;
                let guard = __condvar_wait(cv_ptr, m_ptr);
                c.fetch_add(1, Ordering::SeqCst);
                __mutex_unlock(guard as *mut i8);
            });
            handles.push(h);
        }

        thread::sleep(std::time::Duration::from_millis(100));
        __condvar_notify_all(cv);

        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(counter.load(Ordering::SeqCst), 4);

        __mutex_free(m);
        __condvar_free(cv);
    }
}
