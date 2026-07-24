/**
 * Float bit-level conversion FFI.
 *
 * Provides f64::to_bits() / f64::from_bits() so that the Ruyi stdlib
 * (Buffer) can read/write IEEE 754 double-precision values with full
 * fidelity — no intermediate float arithmetic that loses bits.
 *
 * These are thin wrappers over the standard Rust methods.
 *
 * @author Ruyi Team
 * @date 2026-07-24
 */

/// Reinterpret an `f64` as its IEEE 754 raw bits (as signed i64).
#[no_mangle]
pub extern "C" fn __f64_to_bits(x: f64) -> i64 {
    x.to_bits() as i64
}

/// Reinterpret a signed i64 as the `f64` whose IEEE 754 bit-pattern it represents.
#[no_mangle]
pub extern "C" fn __f64_from_bits(bits: i64) -> f64 {
    f64::from_bits(bits as u64)
}
