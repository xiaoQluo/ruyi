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

thread_local! {
    static TLS_STORE: RefCell<HashMap<i64, i64>> = RefCell::new(HashMap::new());
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

/// Remove a key from the current thread's TLS.
/// Returns the previous value, or 0 if none.
#[no_mangle]
pub extern "C" fn __tls_remove(key: i64) -> i64 {
    TLS_STORE.with(|store| store.borrow_mut().remove(&key).unwrap_or(0))
}

/// Check if a key exists in the current thread's TLS.
/// Returns 1 if exists, 0 otherwise.
#[no_mangle]
pub extern "C" fn __tls_contains(key: i64) -> i8 {
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
