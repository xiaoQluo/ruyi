/**
 * Integration tests for the random FFI bindings exposed from
 * `crates/ruyi_runtime/src/random_ffi.rs`.
 *
 * Each test exercises a single `#[no_mangle] extern "C"` symbol so that
 * linker errors surface as concrete test failures.
 *
 * @author Ruyi Team
 * @date 2026-07-11
 */
use ruyi_runtime::*;

/// `ruyi_random_int(seed, min, max)` MUST return values in the closed
/// interval `[min, max]` for any non-degenerate range.
#[test]
fn test_ruyi_random_int_range() {
    let seed: i64 = 12345;
    for _ in 0..1000 {
        let v = unsafe { ruyi_random_int(seed, 5, 10) };
        assert!(
            v >= 5 && v <= 10,
            "ruyi_random_int(seed, 5, 10) returned out-of-range value: {}",
            v
        );
    }
}

/// `ruyi_random_int(seed, min, max)` MUST return `min` when `min === max`,
/// regardless of how many times it is called.
#[test]
fn test_ruyi_random_int_min_eq_max() {
    let seed: i64 = 12345;
    for _ in 0..100 {
        let v = unsafe { ruyi_random_int(seed, 5, 5) };
        assert_eq!(v, 5, "min==max must collapse to min");
    }
}

/// `ruyi_random_float(seed)` MUST return values in `[0.0, 1.0)`.
#[test]
fn test_ruyi_random_float() {
    let seed: i64 = 12345;
    for _ in 0..1000 {
        let v = unsafe { ruyi_random_float(seed) };
        assert!(
            v >= 0.0 && v < 1.0,
            "ruyi_random_float(seed) returned out-of-range value: {}",
            v
        );
    }
}