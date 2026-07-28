#![allow(clippy::not_unsafe_ptr_arg_deref)]
/**
 * Once FFI — one-time initialisation guard.
 *
 * Implements `std::sync::Once` semantics with a simpler internal
 * representation: an `AtomicBool` flag.  The first thread to call
 * `__once_do()` succeeds; all subsequent callers return immediately
 * without executing the function.
 *
 * Unlike `std::sync::Once`, the caller supplies a C function pointer
 * so the "run once" body is fully controlled by Ruyi codegen.
 *
 * @author Ruyi Team
 * @date 2026-07-25
 */
use std::sync::atomic::{AtomicBool, Ordering};

/// Internal representation of a Once guard.
pub struct Once {
    done: AtomicBool,
}

impl Once {
    fn new() -> Self {
        Once {
            done: AtomicBool::new(false),
        }
    }
}

/// Create a new Once guard.
#[no_mangle]
pub extern "C" fn __once_new() -> *mut Once {
    Box::into_raw(Box::new(Once::new()))
}

/// Execute `fn_ptr(arg)` at most once across all callers.
/// `fn_ptr` is an `extern "C" fn(usize)` — a raw C function pointer.
/// Returns 1 if the function was executed (first caller), 0 otherwise.
///
/// # Safety
/// `fn_ptr` must be a valid, non-null C function pointer.
/// `arg` is passed through to the callback verbatim.
#[no_mangle]
pub extern "C" fn __once_do(ptr: *mut Once, fn_ptr: extern "C" fn(usize), arg: usize) -> i32 {
    if ptr.is_null() || fn_ptr as usize == 0 {
        return 0;
    }
    let once = unsafe { &*ptr };
    if once
        .done
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        fn_ptr(arg);
        1
    } else {
        0
    }
}

/// Check whether the Once guard has already been triggered.
/// Returns 1 if completed, 0 otherwise.
#[no_mangle]
pub extern "C" fn __once_is_completed(ptr: *mut Once) -> i32 {
    if ptr.is_null() {
        return 0;
    }
    let once = unsafe { &*ptr };
    if once.done.load(Ordering::SeqCst) {
        1
    } else {
        0
    }
}

/// Reset the Once guard so the next `__once_do` will execute again.
/// Not thread-safe unless externally synchronized.
#[no_mangle]
pub extern "C" fn __once_reset(ptr: *mut Once) {
    if ptr.is_null() {
        return;
    }
    let once = unsafe { &*ptr };
    once.done.store(false, Ordering::SeqCst);
}

/// Deallocate the Once guard.
#[no_mangle]
pub extern "C" fn __once_free(ptr: *mut Once) {
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

    extern "C" fn increment_counter(arg: usize) {
        let counter = arg as *mut AtomicUsize;
        unsafe {
            (*counter).fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_once_runs_exactly_once() {
        let once = __once_new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_ptr = Arc::as_ptr(&counter) as usize;

        let threads: Vec<_> = (0..10)
            .map(|_| {
                let once_ptr = once as usize;
                let cp = counter_ptr;
                thread::spawn(move || {
                    __once_do(once_ptr as *mut Once, increment_counter, cp);
                })
            })
            .collect();

        for t in threads {
            t.join().unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert_eq!(__once_is_completed(once), 1);
        __once_free(once);
    }

    #[test]
    fn test_once_reset() {
        let once = __once_new();
        let counter = Arc::new(AtomicUsize::new(0));
        let cp = Arc::as_ptr(&counter) as usize;

        __once_do(once, increment_counter, cp);
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        __once_do(once, increment_counter, cp); // should be no-op
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        __once_reset(once);
        __once_do(once, increment_counter, cp);
        assert_eq!(counter.load(Ordering::SeqCst), 2);

        __once_free(once);
    }
}
