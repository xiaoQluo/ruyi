#![allow(clippy::not_unsafe_ptr_arg_deref)]

/**
 * RWLock FFI — read-write lock (shared/exclusive access).
 *
 * Built on `std::sync::RwLock<()>`. The lock itself is heap-allocated
 * and returned as an opaque `*mut i8` handle.
 *
 * Read locks allow concurrent access; write locks are exclusive.
 * The guard is returned as a secondary opaque pointer that must be
 * passed to the corresponding unlock function.
 *
 * @author Ruyi Team
 * @date 2026-07-25
 */
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Create a new unlocked RwLock. Returns an opaque `*mut i8` handle.
#[no_mangle]
pub extern "C" fn __rwlock_new() -> *mut i8 {
    Box::into_raw(Box::new(RwLock::new(()))) as *mut i8
}

/// Acquire a read lock (shared). Multiple readers can hold simultaneously.
/// Returns an opaque guard pointer that MUST be passed to `__rwlock_read_unlock`.
#[no_mangle]
pub extern "C" fn __rwlock_read_lock(ptr: *mut i8) -> *mut i8 {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let rw = unsafe { &*(ptr as *const RwLock<()>) };
    let guard = rw.read().unwrap();
    Box::into_raw(Box::new(guard)) as *mut i8
}

/// Try to acquire a read lock without blocking.
/// Returns guard pointer on success, null if lock is held exclusively.
#[no_mangle]
pub extern "C" fn __rwlock_try_read_lock(ptr: *mut i8) -> *mut i8 {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let rw = unsafe { &*(ptr as *const RwLock<()>) };
    match rw.try_read() {
        Ok(guard) => Box::into_raw(Box::new(guard)) as *mut i8,
        Err(_) => std::ptr::null_mut(),
    }
}

/// Release a read lock guard.
#[no_mangle]
pub extern "C" fn __rwlock_read_unlock(guard_ptr: *mut i8) {
    if guard_ptr.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(guard_ptr as *mut RwLockReadGuard<'_, ()>);
    }
}

/// Acquire a write lock (exclusive). Blocks until available.
/// Returns an opaque guard pointer that MUST be passed to `__rwlock_write_unlock`.
#[no_mangle]
pub extern "C" fn __rwlock_write_lock(ptr: *mut i8) -> *mut i8 {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let rw = unsafe { &*(ptr as *const RwLock<()>) };
    let guard = rw.write().unwrap();
    Box::into_raw(Box::new(guard)) as *mut i8
}

/// Try to acquire a write lock without blocking.
/// Returns guard pointer on success, null if lock is held.
#[no_mangle]
pub extern "C" fn __rwlock_try_write_lock(ptr: *mut i8) -> *mut i8 {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let rw = unsafe { &*(ptr as *const RwLock<()>) };
    match rw.try_write() {
        Ok(guard) => Box::into_raw(Box::new(guard)) as *mut i8,
        Err(_) => std::ptr::null_mut(),
    }
}

/// Release a write lock guard.
#[no_mangle]
pub extern "C" fn __rwlock_write_unlock(guard_ptr: *mut i8) {
    if guard_ptr.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(guard_ptr as *mut RwLockWriteGuard<'_, ()>);
    }
}

/// Deallocate the RwLock. Must NOT be locked when called.
#[no_mangle]
pub extern "C" fn __rwlock_free(ptr: *mut i8) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ptr as *mut RwLock<()>);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_read_lock_unlock() {
        let rw = __rwlock_new();
        assert!(!rw.is_null());

        let guard = __rwlock_read_lock(rw);
        assert!(!guard.is_null());
        __rwlock_read_unlock(guard);

        __rwlock_free(rw);
    }

    #[test]
    fn test_write_lock_unlock() {
        let rw = __rwlock_new();
        let guard = __rwlock_write_lock(rw);
        assert!(!guard.is_null());
        __rwlock_write_unlock(guard);
        __rwlock_free(rw);
    }

    #[test]
    fn test_multiple_readers() {
        let rw = __rwlock_new();
        let g1 = __rwlock_read_lock(rw);
        let g2 = __rwlock_read_lock(rw);
        assert!(!g1.is_null());
        assert!(!g2.is_null());
        __rwlock_read_unlock(g1);
        __rwlock_read_unlock(g2);
        __rwlock_free(rw);
    }

    #[test]
    fn test_try_write_fails_during_read() {
        let rw = __rwlock_new();
        let _g = __rwlock_read_lock(rw);
        let w = __rwlock_try_write_lock(rw);
        assert!(w.is_null(), "try_write should fail during read");
        __rwlock_read_unlock(_g);
        __rwlock_free(rw);
    }

    #[test]
    fn test_concurrent_readers() {
        let rw = __rwlock_new();
        let rw_ptr = rw as usize;

        let handle = thread::spawn(move || {
            let rw2 = rw_ptr as *mut i8;
            let g = __rwlock_read_lock(rw2);
            assert!(!g.is_null());
            thread::sleep(std::time::Duration::from_millis(10));
            __rwlock_read_unlock(g);
        });

        let g = __rwlock_read_lock(rw);
        assert!(!g.is_null());
        __rwlock_read_unlock(g);

        handle.join().unwrap();
        __rwlock_free(rw);
    }
}
