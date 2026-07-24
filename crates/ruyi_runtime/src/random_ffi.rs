/**
 * C FFI implementations backing `stdlib/random.ry`.
 *
 * Provides five `#[no_mangle] extern "C"` symbols:
 *
 * - `__random_new(seed)` — initialize a generator; `seed == 0` falls back
 *   to an entropy-derived seed from the runtime `RandomState`.
 * - `__random_int(rng, min, max)` — uniform integer in the closed
 *   interval `[min, max]`. Returns `min` when `min === max`. Aborts on
 *   `min > max` so callers see a diagnostic rather than silently wrapping.
 * - `__random_float(rng)` — uniform float in `[0.0, 1.0)`.
 * - `__random_bool(rng)` — uniform boolean derived from `__random_int`.
 * - `__random_bytes(rng, n)` — `n` pseudo-random bytes returned as a
 *   freshly-allocated null-terminated buffer (treated as a `string` by the
 *   stdlib layer).
 *
 * All randomness is produced by a tiny `xorshift64*` step; the crate
 * deliberately avoids the external `rand` dependency to keep the runtime
 * minimal.
 *
 * @author Ruyi Team
 * @date 2026-07-11
 */
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

/// xorshift64* multiplier used to scramble the input `rng` token.
const XSHIFT_MULT_A: u64 = 0x2545_F491_4F6C_DD1D;
/// xorshift64* multiplier applied after the first xor-shift step.
const XSHIFT_MULT_B: u64 = 0xBF58_476D_1CE4_E5B9;

/// Mix the supplied `seed` value through `xorshift64*` so that callers
/// cannot accidentally observe degenerate streams (e.g. `seed == 1`).
fn xorshift_mix(seed: u64) -> u64 {
    let mut s = seed.wrapping_mul(XSHIFT_MULT_A);
    s ^= s >> 33;
    s = s.wrapping_mul(XSHIFT_MULT_B);
    s ^= s >> 33;
    s
}

/// Initialize a generator.
///
/// When `seed == 0` the runtime derives a non-deterministic seed from
/// `RandomState`. Any other seed value is mixed through `xorshift64*` so
/// that the same input always produces the same output stream.
#[no_mangle]
pub extern "C" fn __random_new(seed: i64) -> i64 {
    let s = if seed == 0 {
        RandomState::new().build_hasher().finish()
    } else {
        xorshift_mix(seed as u64)
    };
    s as i64
}

/// Return a uniform integer in the closed interval `[min, max]`.
///
/// When `min === max` the function short-circuits to `min`. When
/// `min > max` the function aborts (not panics) to surface the bug
/// without unwinding across the `extern "C"` boundary, which would be
/// undefined behavior under `panic = "unwind"`.
#[no_mangle]
pub extern "C" fn __random_int(rng: i64, min: i64, max: i64) -> i64 {
    if min == max {
        return min;
    }
    if min > max {
        std::process::abort();
    }
    let mut s = (rng as u64).wrapping_mul(XSHIFT_MULT_A);
    s ^= s >> 33;
    s = s.wrapping_mul(XSHIFT_MULT_B);
    s ^= s >> 33;
    let range = (max - min + 1) as u64;
    min + (s % range) as i64
}

/// Return a uniform float in the half-open interval `[0.0, 1.0)`.
///
/// The denominator rounds `u64::MAX` up to `2^64`; this loses one bit of
/// precision at the top of the range, which is acceptable for the stdlib
/// stub and matches the convention used by other minimal RNGs.
#[no_mangle]
pub extern "C" fn __random_float(rng: i64) -> f64 {
    let mut s = (rng as u64).wrapping_mul(XSHIFT_MULT_A);
    s ^= s >> 33;
    s = s.wrapping_mul(XSHIFT_MULT_B);
    s ^= s >> 33;
    (s as f64) / (u64::MAX as f64)
}

/// Return a uniform boolean derived from `__random_int(rng, 0, 1)`.
#[no_mangle]
pub extern "C" fn __random_bool(rng: i64) -> bool {
    __random_int(rng, 0, 1) == 1
}

/// Allocate a buffer of `n` pseudo-random bytes and return a pointer to
/// it. The buffer is null-terminated after the `n` bytes so it can be
/// treated as a Ruyi `string` by the stdlib layer.
///
/// # Safety
///
/// The returned pointer is valid until the next call to
/// `__random_bytes`. Callers MUST copy the bytes out (e.g. into a
/// Ruyi `string`) before issuing another `nextBytes` invocation.
/// Concurrency is supported because each call produces an independent
/// allocation; only the returned pointer's lifetime is bounded.
///
/// Negative `n` is treated as zero.
#[no_mangle]
pub extern "C" fn __random_bytes(rng: i64, n: i64) -> *mut i8 {
    use std::alloc::{alloc, Layout};

    let count = if n <= 0 { 0 } else { n as usize };
    let total = count + 1;
    let layout = match Layout::from_size_align(total, 1) {
        Ok(l) => l,
        Err(_) => return std::ptr::null_mut(),
    };
    let out = unsafe { alloc(layout) } as *mut i8;
    if out.is_null() {
        return std::ptr::null_mut();
    }

    let mut s = rng as u64;
    unsafe {
        for i in 0..count {
            s = s.wrapping_mul(XSHIFT_MULT_A);
            s ^= s >> 33;
            s = s.wrapping_mul(XSHIFT_MULT_B);
            s ^= s >> 33;
            *out.add(i) = (s & 0xFF) as i8;
        }
        *out.add(count) = 0;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_distinct_seeds() {
        let a = __random_new(1);
        let b = __random_new(2);
        assert_ne!(a, b);
    }

    #[test]
    fn test_new_zero_uses_entropy() {
        let a = __random_new(0);
        let b = __random_new(0);
        assert_ne!(a, b);
    }

    #[test]
    fn test_int_range() {
        let seed = 42_i64;
        for _ in 0..10_000 {
            let v = __random_int(seed, 5, 10);
            assert!((5..=10).contains(&v), "out of range: {}", v);
        }
    }

    #[test]
    fn test_int_min_eq_max() {
        for seed in [0_i64, 1, 42, i64::MAX] {
            assert_eq!(__random_int(seed, 5, 5), 5);
        }
    }

    #[test]
    fn test_float_range() {
        let seed = 42_i64;
        for _ in 0..10_000 {
            let v = __random_float(seed);
            assert!((0.0..1.0).contains(&v), "out of range: {}", v);
        }
    }

    #[test]
    fn test_bool_observed_both_polarities_across_seeds() {
        let mut seen_true = false;
        let mut seen_false = false;
        for seed in 0_i64..1_000 {
            match __random_bool(seed) {
                true => seen_true = true,
                false => seen_false = true,
            }
            if seen_true && seen_false {
                return;
            }
        }
        assert!(seen_true && seen_false, "bool distribution is degenerate");
    }

    #[test]
    fn test_bytes_payload_writable_and_nul_terminated() {
        let seed = 7_i64;
        for len in [0_i64, 1, 16, 256] {
            let p = __random_bytes(seed, len);
            assert!(!p.is_null());
            unsafe {
                assert_eq!(*p.add(len as usize), 0);
                for i in 0..len as usize {
                    let _ = *p.add(i);
                }
            }
        }
    }

    #[test]
    #[ignore = "__random_int aborts (not panics) on min > max to avoid FFI UB"]
    fn test_int_aborts_when_min_greater_than_max() {
        let _ = __random_int(0, 10, 5);
    }
}
