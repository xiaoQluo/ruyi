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

// ============================================================
// Inverse Trigonometric Functions
// ============================================================

/// Arc-cosine of x (inverse cosine).
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_acos(x: f64) -> f64 {
    x.acos()
}

/// Arc-sine of x (inverse sine).
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_asin(x: f64) -> f64 {
    x.asin()
}

/// Arc-tangent of x (inverse tangent).
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_atan(x: f64) -> f64 {
    x.atan()
}

/// Four-quadrant arc-tangent of y/x.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

// ============================================================
// Logarithmic and Exponential Functions
// ============================================================

/// Base-2 logarithm of x.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_log2(x: f64) -> f64 {
    x.log2()
}

/// Base-10 logarithm of x.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_log10(x: f64) -> f64 {
    x.log10()
}

/// Exponential function (e^x).
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_exp(x: f64) -> f64 {
    x.exp()
}

// ============================================================
// Sign and Truncation
// ============================================================

/// Sign of x: -1 for negative, 1 for positive or positive zero.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_sign(x: f64) -> f64 {
    x.signum()
}

/// Integer part of x (truncation toward zero).
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_trunc(x: f64) -> f64 {
    x.trunc()
}

// ============================================================
// Hyperbolic Functions
// ============================================================

/// Hyperbolic sine of x.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_sinh(x: f64) -> f64 {
    x.sinh()
}

/// Hyperbolic cosine of x.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_cosh(x: f64) -> f64 {
    x.cosh()
}

/// Hyperbolic tangent of x.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_tanh(x: f64) -> f64 {
    x.tanh()
}

// ============================================================
// Miscellaneous Math
// ============================================================

/// Square root of sum of squares (hypotenuse): sqrt(x^2 + y^2).
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_hypot(x: f64, y: f64) -> f64 {
    x.hypot(y)
}

/// Cube root of x.
///
/// # Safety
/// None - pure function with no pointer arguments.
#[no_mangle]
pub extern "C" fn __math_cbrt(x: f64) -> f64 {
    x.cbrt()
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

    #[test]
    fn test_inverse_trig() {
        assert!((__math_acos(1.0) - 0.0).abs() < 1e-10);
        assert!((__math_asin(0.0) - 0.0).abs() < 1e-10);
        assert!((__math_atan(0.0) - 0.0).abs() < 1e-10);
        assert!((__math_atan2(0.0, 1.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_hyperbolic() {
        assert!((__math_sinh(0.0) - 0.0).abs() < 1e-10);
        assert!((__math_cosh(0.0) - 1.0).abs() < 1e-10);
        assert!((__math_tanh(0.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_log2_log10_exp() {
        assert!((__math_log2(1.0) - 0.0).abs() < 1e-10);
        assert!((__math_log10(1.0) - 0.0).abs() < 1e-10);
        assert!((__math_exp(0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_sign_trunc_hypot_cbrt() {
        assert_eq!(__math_sign(5.0), 1.0);
        assert_eq!(__math_sign(-5.0), -1.0);
        assert_eq!(__math_trunc(2.7), 2.0);
        assert!((__math_hypot(3.0, 4.0) - 5.0).abs() < 1e-10);
        assert!((__math_cbrt(8.0) - 2.0).abs() < 1e-10);
    }
}
