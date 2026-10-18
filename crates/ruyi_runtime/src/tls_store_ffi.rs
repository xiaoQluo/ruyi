#![allow(clippy::not_unsafe_ptr_arg_deref)]

/**
 * Thread-Local Storage FFI.
 *
 * Provides per-thread key-value storage backed by Rust's `thread_local!`
 * macro. Each thread has its own independent copy of stored values.
 *
 * Uses a `thread_local! { RefCell<HashMap<i64, i64>> }` to map
 * integer keys to integer values, allowing the Ruyi stdlib to
 * build typed wrappers (ThreadLocalInt, ThreadLocalBool, etc.).
 *
 * @author Ruyi Team
 * @date 2026-07-25
 */
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

thread_local! {
    static TLS_STORE: RefCell<HashMap<i64, i64>> = RefCell::new(HashMap::new());
    /// String values are copied into thread-local heap memory so the
    /// returned pointer stays valid for the lifetime of the entry.
    static TLS_STRINGS: RefCell<HashMap<i64, CString>> = RefCell::new(HashMap::new());
}

/// Store a value for the given key in the current thread's TLS.
/// Returns the previous value, or 0 if none.
#[no_mangle]
pub extern "C" fn __tls_store(key: i64, value: i64) -> i64 {
    TLS_STORE.with(|store| store.borrow_mut().insert(key, value).unwrap_or(0))
}

/// Load the value for the given key in the current thread's TLS.
/// Returns 0 if the key is not set.
#[no_mangle]
pub extern "C" fn __tls_load(key: i64) -> i64 {
    TLS_STORE.with(|store| store.borrow().get(&key).copied().unwrap_or(0))
}

/// Store a string value for the given key in the current thread's TLS.
///
/// The value is copied, so the caller's buffer may be released; the
/// pointer returned by `__tls_load_str` stays valid until the entry is
/// removed or replaced.
///
/// # Safety
/// `value` must be null or a valid null-terminated C string.
#[no_mangle]
pub extern "C" fn __tls_store_str(key: i64, value: *const c_char) {
    let text = if value.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned()
    };
    let owned = CString::new(text).unwrap_or_else(|_| CString::new("").unwrap());
    TLS_STRINGS.with(|store| {
        store.borrow_mut().insert(key, owned);
    });
}

/// Load a string value for the given key in the current thread's TLS.
/// Returns null when the key is not set.
#[no_mangle]
pub extern "C" fn __tls_load_str(key: i64) -> *mut c_char {
    TLS_STRINGS.with(|store| {
        store
            .borrow()
            .get(&key)
            .map(|s| s.as_ptr() as *mut c_char)
            .unwrap_or(std::ptr::null_mut())
    })
}

/// Remove a key from the current thread's TLS.
/// Returns the previous value, or 0 if none.
#[no_mangle]
pub extern "C" fn __tls_remove(key: i64) -> i64 {
    TLS_STRINGS.with(|store| {
        store.borrow_mut().remove(&key);
    });
    TLS_STORE.with(|store| store.borrow_mut().remove(&key).unwrap_or(0))
}

/// Check if a key exists in the current thread's TLS.
/// Returns 1 if exists, 0 otherwise.
#[no_mangle]
pub extern "C" fn __tls_contains(key: i64) -> i8 {
    let has_string = TLS_STRINGS.with(|store| store.borrow().contains_key(&key));
    if has_string {
        return 1;
    }
    TLS_STORE.with(|store| {
        if store.borrow().contains_key(&key) {
            1
        } else {
            0
        }
    })
}

/// Clear all TLS entries for the current thread.
#[no_mangle]
pub extern "C" fn __tls_clear() {
    TLS_STRINGS.with(|store| {
        store.borrow_mut().clear();
    });
    TLS_STORE.with(|store| {
        store.borrow_mut().clear();
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_store_and_load() {
        __tls_store(1, 42);
        assert_eq!(__tls_load(1), 42);
        assert_eq!(__tls_load(999), 0);
        __tls_remove(1);
    }

    #[test]
    fn test_store_returns_previous() {
        let prev = __tls_store(2, 10);
        assert_eq!(prev, 0);
        let prev = __tls_store(2, 20);
        assert_eq!(prev, 10);
        __tls_remove(2);
    }

    #[test]
    fn test_contains() {
        assert_eq!(__tls_contains(3), 0);
        __tls_store(3, 100);
        assert_eq!(__tls_contains(3), 1);
        __tls_remove(3);
        assert_eq!(__tls_contains(3), 0);
    }

    #[test]
    fn test_thread_isolation() {
        __tls_store(1, 100);

        let handle = thread::spawn(|| {
            // New thread — should see empty TLS
            assert_eq!(__tls_load(1), 0);
            __tls_store(1, 200);
            assert_eq!(__tls_load(1), 200);
        });
        handle.join().unwrap();

        // Main thread — unchanged
        assert_eq!(__tls_load(1), 100);
        __tls_remove(1);
    }

    #[test]
    fn test_clear() {
        __tls_store(1, 1);
        __tls_store(2, 2);
        __tls_clear();
        assert_eq!(__tls_load(1), 0);
        assert_eq!(__tls_load(2), 0);
    }
}
