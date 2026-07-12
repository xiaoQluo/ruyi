/**
 * C FFI implementations backing `stdlib/math.ry`.
 *
 * Provides math functions via C ABI for use by Ruyi standard library.
 * All functions wrap Rust's standard math library functions.
 *
 * @author Ruyi Team
 * @date 2026-07-12
 */

/// Square root of x.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_sqrt(x: f64) -> f64 {
    x.sqrt()
}

/// x raised to the power of y.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_pow(x: f64, y: f64) -> f64 {
    x.powf(y)
}

/// Absolute value of x.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_abs(x: f64) -> f64 {
    x.abs()
}

/// Minimum of two values.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_min(a: f64, b: f64) -> f64 {
    a.min(b)
}

/// Maximum of two values.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_max(a: f64, b: f64) -> f64 {
    a.max(b)
}

/// Sine of x (radians).
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_sin(x: f64) -> f64 {
    x.sin()
}

/// Cosine of x (radians).
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_cos(x: f64) -> f64 {
    x.cos()
}

/// Tangent of x (radians).
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_tan(x: f64) -> f64 {
    x.tan()
}

/// Natural logarithm of x.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_log(x: f64) -> f64 {
    x.ln()
}

/// Ceiling of x (smallest integer >= x).
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_ceil(x: f64) -> f64 {
    x.ceil()
}

/// Floor of x (largest integer <= x).
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_floor(x: f64) -> f64 {
    x.floor()
}

/// Round x to the nearest integer.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_round(x: f64) -> f64 {
    x.round()
}

/// Mathematical constant PI (π).
#[no_mangle]
pub extern "C" fn __math_pi() -> f64 {
    std::f64::consts::PI
}

/// Mathematical constant E (euler's number).
#[no_mangle]
pub extern "C" fn __math_e() -> f64 {
    std::f64::consts::E
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqrt() {
        assert_eq!(__math_sqrt(4.0), 2.0);
        assert_eq!(__math_sqrt(9.0), 3.0);
        assert_eq!(__math_sqrt(0.0), 0.0);
    }

    #[test]
    fn test_pow() {
        assert_eq!(__math_pow(2.0, 3.0), 8.0);
        assert_eq!(__math_pow(10.0, 2.0), 100.0);
        assert_eq!(__math_pow(5.0, 0.0), 1.0);
    }

    #[test]
    fn test_abs() {
        assert_eq!(__math_abs(-5.0), 5.0);
        assert_eq!(__math_abs(5.0), 5.0);
        assert_eq!(__math_abs(0.0), 0.0);
    }

    #[test]
    fn test_min_max() {
        assert_eq!(__math_min(3.0, 5.0), 3.0);
        assert_eq!(__math_max(3.0, 5.0), 5.0);
        assert_eq!(__math_min(5.0, 3.0), 3.0);
        assert_eq!(__math_max(5.0, 3.0), 5.0);
    }

    #[test]
    fn test_trig() {
        assert!((__math_sin(0.0) - 0.0).abs() < 1e-10);
        assert!((__math_cos(0.0) - 1.0).abs() < 1e-10);
        assert!((__math_tan(0.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_log() {
        assert!((__math_log(1.0) - 0.0).abs() < 1e-10);
        assert!((__math_log(std::f64::consts::E) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ceil_floor_round() {
        assert_eq!(__math_ceil(2.3), 3.0);
        assert_eq!(__math_ceil(2.7), 3.0);
        assert_eq!(__math_floor(2.3), 2.0);
        assert_eq!(__math_floor(2.7), 2.0);
        assert_eq!(__math_round(2.3), 2.0);
        assert_eq!(__math_round(2.7), 3.0);
    }

    #[test]
    fn test_constants() {
        assert!((__math_pi() - std::f64::consts::PI).abs() < 1e-10);
        assert!((__math_e() - std::f64::consts::E).abs() < 1e-10);
    }
}