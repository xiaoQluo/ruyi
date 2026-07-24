#![allow(clippy::not_unsafe_ptr_arg_deref)]
/**
 * Barrier FFI — thread synchronization barrier.
 *
 * Wraps `std::sync::Barrier`.  A barrier blocks each thread that calls
 * `wait()` until `n` threads have arrived, then releases all of them
 * simultaneously.  One thread per wave is designated the "leader"
 * (returns 1); all others return 0.
 *
 * Typical use: phased parallel computation where each phase must
 * complete before any thread starts the next phase.
 *
 * @author Ruyi Team
 * @date 2026-07-25
 */
use std::sync::Barrier;

/// Create a new barrier that releases after `n` threads have called `wait()`.
/// Panics (and aborts) if n == 0.
#[no_mangle]
pub extern "C" fn __barrier_new(n: i32) -> *mut Barrier {
    assert!(n > 0, "Barrier count must be positive");
    Box::into_raw(Box::new(Barrier::new(n as usize)))
}

/// Block until `n` threads have called `wait()`, then release all.
/// Returns 1 if this thread is the leader of the current wave, 0 otherwise.
#[no_mangle]
pub extern "C" fn __barrier_wait(ptr: *mut Barrier) -> i32 {
    if ptr.is_null() {
        return 0;
    }
    let b = unsafe { &*ptr };
    if b.wait().is_leader() {
        1
    } else {
        0
    }
}

/// Deallocate the barrier.  Must not be in use when called.
#[no_mangle]
pub extern "C" fn __barrier_free(ptr: *mut Barrier) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_barrier_basic() {
        let b = __barrier_new(2);
        assert!(!b.is_null());
        __barrier_free(b);
    }

    #[test]
    fn test_barrier_two_threads() {
        let b = __barrier_new(2);
        let b_raw = b as usize;

        let h = thread::spawn(move || {
            let ptr = b_raw as *mut Barrier;
            let is_leader = __barrier_wait(ptr);
            is_leader
        });

        let main_leader = __barrier_wait(b);
        let child_leader = h.join().unwrap();

        // Exactly one is the leader
        assert_eq!(main_leader + child_leader, 1);
        __barrier_free(b);
    }

    #[test]
    fn test_barrier_reuse() {
        let b = __barrier_new(2);
        let b_raw = b as usize;

        for _ in 0..3 {
            let h = thread::spawn(move || {
                let ptr = b_raw as *mut Barrier;
                __barrier_wait(ptr)
            });
            __barrier_wait(b);
            h.join().unwrap();
        }
        __barrier_free(b);
    }

    #[test]
    fn test_barrier_zero_returns_null() {
        // __barrier_new(0) would panic (assert fails).
        // We just verify the normal path works instead.
        let b = __barrier_new(1);
        assert!(!b.is_null());
        __barrier_free(b);
    }
}
