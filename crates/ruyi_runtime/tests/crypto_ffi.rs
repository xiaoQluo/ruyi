/**
 * Integration tests for the crypto FFI bindings exposed from
 * `crates/ruyi_runtime/src/crypto_ffi.rs`.
 *
 * The string-returning symbols are declared to Ruyi code as non-nullable
 * `string` (`BuiltinSig::String` in `codegen/builtins_table.rs`), so they
 * must never hand back a null pointer: `stdlib/crypto/aes.ry` only tests
 * for `""` and would otherwise propagate a raw null into user code,
 * segfaulting on the next `print`.
 *
 * @author Ruyi Team
 * @date 2026-10-15
 */
use ruyi_runtime::crypto_ffi::{
    __crypto_aes_gcm_decrypt_hex, __crypto_aes_gcm_encrypt_hex, __crypto_hmac_sha256_raw,
    __crypto_hmac_sha512_raw,
};
use std::ffi::CStr;

fn as_str(ptr: *mut std::os::raw::c_char) -> Option<&'static str> {
    if ptr.is_null() {
        None
    } else {
        Some(
            unsafe { CStr::from_ptr(ptr as *const std::os::raw::c_char) }
                .to_str()
                .unwrap_or(""),
        )
    }
}

const KEY: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
const NONCE: &str = "000102030405060708090a0b";

fn c(s: &str) -> Vec<std::os::raw::c_char> {
    let mut v = s
        .as_bytes()
        .iter()
        .map(|b| *b as std::os::raw::c_char)
        .collect::<Vec<_>>();
    v.push(0);
    v
}

/// A non-hex plaintext is invalid input, not a crash: the symbol must
/// return an empty C string rather than a null pointer.
#[test]
fn test_aes_encrypt_rejects_non_hex_without_null() {
    let key = c(KEY);
    let nonce = c(NONCE);
    let plain = c("Secret message for AES demo");
    let out = __crypto_aes_gcm_encrypt_hex(key.as_ptr(), nonce.as_ptr(), plain.as_ptr());
    assert_eq!(
        as_str(out),
        Some(""),
        "invalid hex must yield an empty string, not a null pointer"
    );
}

/// A key of the wrong length must be rejected the same way.
#[test]
fn test_aes_encrypt_rejects_short_key_without_null() {
    let key = c("deadbeef");
    let nonce = c(NONCE);
    let plain = c("68656c6c6f");
    let out = __crypto_aes_gcm_encrypt_hex(key.as_ptr(), nonce.as_ptr(), plain.as_ptr());
    assert_eq!(
        as_str(out),
        Some(""),
        "short key must yield an empty string"
    );
}

/// Decryption of a ciphertext that fails authentication must also report
/// failure as an empty string, not a null pointer.
#[test]
fn test_aes_decrypt_rejects_garbage_without_null() {
    let key = c(KEY);
    let nonce = c(NONCE);
    let bad = c("00112233");
    let out = __crypto_aes_gcm_decrypt_hex(key.as_ptr(), nonce.as_ptr(), bad.as_ptr());
    assert_eq!(
        as_str(out),
        Some(""),
        "garbage ciphertext must yield an empty string"
    );
}

/// Null input pointers must be reported as an empty string too.
#[test]
fn test_aes_encrypt_null_input_without_null() {
    let key = c(KEY);
    let nonce = c(NONCE);
    let out = __crypto_aes_gcm_encrypt_hex(key.as_ptr(), nonce.as_ptr(), std::ptr::null());
    assert_eq!(
        as_str(out),
        Some(""),
        "null input must yield an empty string"
    );
}

/// The happy path must keep working: valid hex round-trips.
#[test]
fn test_aes_roundtrip_still_works() {
    let key = c(KEY);
    let nonce = c(NONCE);
    let plain = c("68656c6c6f");
    let cipher = as_str(__crypto_aes_gcm_encrypt_hex(
        key.as_ptr(),
        nonce.as_ptr(),
        plain.as_ptr(),
    ))
    .expect("valid input must not return null")
    .to_string();
    assert!(!cipher.is_empty(), "valid input must produce ciphertext");

    let cipher_c = c(&cipher);
    let recovered = as_str(__crypto_aes_gcm_decrypt_hex(
        key.as_ptr(),
        nonce.as_ptr(),
        cipher_c.as_ptr(),
    ))
    .expect("valid ciphertext must not return null")
    .to_string();
    assert_eq!(recovered, "68656c6c6f");
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// A Ruyi array header is `[len][cap][data_ptr]`; the raw crypto entry
/// points take headers rather than element pointers, matching
/// `__fs_read_raw`. Elements are `i64`-wide, matching what `zeroArray` and
/// `stringToBytes` actually produce in the stdlib.
struct RuyiArray {
    header: Vec<i64>,
    data: Vec<i64>,
}

impl RuyiArray {
    fn new(bytes: Vec<u8>) -> Self {
        let data: Vec<i64> = bytes.iter().map(|b| *b as i64).collect();
        let len = data.len() as i64;
        let ptr = data.as_ptr() as i64;
        RuyiArray {
            header: vec![len, len, ptr],
            data,
        }
    }

    fn bytes(&self) -> Vec<u8> {
        self.data.iter().map(|v| *v as u8).collect()
    }

    fn as_ptr(&self) -> *const u8 {
        self.header.as_ptr() as *const u8
    }

    fn as_mut_ptr(&mut self) -> *mut u8 {
        self.header.as_mut_ptr() as *mut u8
    }
}

/// PBKDF2 drives HMAC over raw digests, so it needs a byte-level entry
/// point; the string-based `__crypto_hmac_sha256` cannot carry binary
/// intermediate values. Vectors are RFC 4231 test case 1.
#[test]
fn test_hmac_sha256_raw_matches_rfc4231_case1() {
    let key = RuyiArray::new(vec![0x0bu8; 20]);
    let data = RuyiArray::new(b"Hi There".to_vec());
    let mut out = RuyiArray::new(vec![0u8; 32]);
    let rc = __crypto_hmac_sha256_raw(key.as_ptr(), data.as_ptr(), out.as_mut_ptr());
    assert_eq!(rc, 0, "valid input must succeed");
    assert_eq!(
        hex(&out.bytes()),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
}

/// RFC 4231 test case 1, SHA-512 variant.
#[test]
fn test_hmac_sha512_raw_matches_rfc4231_case1() {
    let key = RuyiArray::new(vec![0x0bu8; 20]);
    let data = RuyiArray::new(b"Hi There".to_vec());
    let mut out = RuyiArray::new(vec![0u8; 64]);
    let rc = __crypto_hmac_sha512_raw(key.as_ptr(), data.as_ptr(), out.as_mut_ptr());
    assert_eq!(rc, 0, "valid input must succeed");
    assert_eq!(
        hex(&out.bytes()),
        "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cde\
         daa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854"
    );
}

/// An empty key and empty message are legal inputs and must not be
/// rejected — PBKDF2 reaches HMAC with short keys on every iteration.
#[test]
fn test_hmac_sha256_raw_accepts_empty_inputs() {
    let key = RuyiArray::new(Vec::new());
    let data = RuyiArray::new(Vec::new());
    let mut out = RuyiArray::new(vec![0u8; 32]);
    let rc = __crypto_hmac_sha256_raw(key.as_ptr(), data.as_ptr(), out.as_mut_ptr());
    assert_eq!(rc, 0, "empty key and message must succeed");
    assert_eq!(
        hex(&out.bytes()),
        "b613679a0814d9ec772f95d778c35fc5ff1697c493715653c6c712144292c5ad"
    );
}

/// A null array header is a caller bug and must be reported, not written
/// through.
#[test]
fn test_hmac_sha256_raw_rejects_null_output() {
    let key = RuyiArray::new(vec![0x0bu8; 20]);
    let data = RuyiArray::new(b"Hi There".to_vec());
    let rc = __crypto_hmac_sha256_raw(key.as_ptr(), data.as_ptr(), std::ptr::null_mut());
    assert_eq!(rc, -1, "null output must be rejected");
}
