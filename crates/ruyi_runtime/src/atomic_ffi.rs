/**
 * Atomic FFI — thread-safe integer operations.
 *
 * Thin C-ABI wrappers over `std::sync::atomic::AtomicI64` allocated on
 * the heap via `Box::into_raw`.  Each returned `*mut i8` is an opaque
 * handle that the caller must eventually pass to `__atomic_i64_free`.
 *
 * All operations use `Ordering::SeqCst` — the strongest ordering,
 * matching the convention already established by `c_exports.rs`.
 *
 * @author Ruyi Team
 * @date 2026-07-24
 */
use std::sync::atomic::{AtomicI64, Ordering};

/// Allocate a new AtomicI64 initialised to `value`.
/// Returns an opaque `*mut i8` handle (never null).
#[no_mangle]
pub extern "C" fn __atomic_i64_new(value: i64) -> *mut i8 {
    Box::into_raw(Box::new(AtomicI64::new(value))) as *mut i8
}

/// Atomically load the current value.
#[no_mangle]
pub extern "C" fn __atomic_i64_load(ptr: *mut i8) -> i64 {
    if ptr.is_null() {
        return 0;
    }
    unsafe { &*(ptr as *const AtomicI64) }.load(Ordering::SeqCst)
}

/// Atomically store a new value.
#[no_mangle]
pub extern "C" fn __atomic_i64_store(ptr: *mut i8, value: i64) {
    if ptr.is_null() {
        return;
    }
    unsafe { &*(ptr as *const AtomicI64) }.store(value, Ordering::SeqCst);
}

/// Compare-and-swap: if `*ptr == expected`, store `desired`.
/// Returns the **previous** value (identical to `expected` on success).
#[no_mangle]
pub extern "C" fn __atomic_i64_cas(ptr: *mut i8, expected: i64, desired: i64) -> i64 {
    if ptr.is_null() {
        return 0;
    }
    unsafe { &*(ptr as *const AtomicI64) }
        .compare_exchange(expected, desired, Ordering::SeqCst, Ordering::SeqCst)
        .unwrap_or_else(|prev| prev)
}

/// Atomically add `value` to `*ptr`.  Returns the **previous** value.
#[no_mangle]
pub extern "C" fn __atomic_i64_fetch_add(ptr: *mut i8, value: i64) -> i64 {
    if ptr.is_null() {
        return 0;
    }
    unsafe { &*(ptr as *const AtomicI64) }.fetch_add(value, Ordering::SeqCst)
}

/// Atomically subtract `value` from `*ptr`.  Returns the **previous** value.
#[no_mangle]
pub extern "C" fn __atomic_i64_fetch_sub(ptr: *mut i8, value: i64) -> i64 {
    if ptr.is_null() {
        return 0;
    }
    unsafe { &*(ptr as *const AtomicI64) }.fetch_sub(value, Ordering::SeqCst)
}

/// Atomically swap the value.  Returns the **previous** value.
#[no_mangle]
pub extern "C" fn __atomic_i64_swap(ptr: *mut i8, value: i64) -> i64 {
    if ptr.is_null() {
        return 0;
    }
    unsafe { &*(ptr as *const AtomicI64) }.swap(value, Ordering::SeqCst)
}

/// Deallocate the atomic value previously returned by `__atomic_i64_new`.
#[no_mangle]
pub extern "C" fn __atomic_i64_free(ptr: *mut i8) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ptr as *mut AtomicI64);
    }
}
