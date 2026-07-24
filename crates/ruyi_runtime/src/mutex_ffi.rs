/**
 * Mutex FFI — basic mutual-exclusion lock.
 *
 * Thin C-ABI wrappers over `std::sync::Mutex<()>`.  The mutex itself is
 * heap-allocated; `__mutex_lock` returns a *secondary* opaque pointer
 * (the guard) that must be passed to `__mutex_unlock` to release the lock.
 *
 * All functions are `#[no_mangle] pub extern "C"` so Ruyi codegen can
 * link them directly.
 *
 * @author Ruyi Team
 * @date 2026-07-24
 */
use std::sync::Mutex;

/// Create a new unlocked mutex.  Returns an opaque `*mut i8` handle.
#[no_mangle]
pub extern "C" fn __mutex_new() -> *mut i8 {
    Box::into_raw(Box::new(Mutex::new(()))) as *mut i8
}

/// Acquire the mutex (blocks until available).
/// Returns an opaque guard pointer that MUST be passed to `__mutex_unlock`.
#[no_mangle]
pub extern "C" fn __mutex_lock(ptr: *mut i8) -> *mut i8 {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let m = unsafe { &*(ptr as *const Mutex<()>) };
    let guard = m.lock().unwrap();
    Box::into_raw(Box::new(guard)) as *mut i8
}

/// Release a mutex guard previously returned by `__mutex_lock`.
#[no_mangle]
pub extern "C" fn __mutex_unlock(guard_ptr: *mut i8) {
    if guard_ptr.is_null() {
        return;
    }
    unsafe {
        // Reconstruct the boxed MutexGuard and drop it to release the lock.
        let _ = Box::from_raw(guard_ptr as *mut std::sync::MutexGuard<'_, ()>);
    }
}

/// Try to acquire the mutex without blocking.
/// Returns an opaque guard pointer on success, or null if the lock is held.
#[no_mangle]
pub extern "C" fn __mutex_try_lock(ptr: *mut i8) -> *mut i8 {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let m = unsafe { &*(ptr as *const Mutex<()>) };
    match m.try_lock() {
        Ok(guard) => Box::into_raw(Box::new(guard)) as *mut i8,
        Err(_) => std::ptr::null_mut(),
    }
}

/// Deallocate a mutex.  Must NOT be locked when called.
#[no_mangle]
pub extern "C" fn __mutex_free(ptr: *mut i8) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ptr as *mut Mutex<()>);
    }
}
