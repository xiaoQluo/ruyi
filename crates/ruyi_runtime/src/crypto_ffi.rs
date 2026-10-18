#![allow(clippy::not_unsafe_ptr_arg_deref)]

/**
 * Crypto FFI — hashing, HMAC, AES-256-GCM, X25519.
 *
 * Two API layers:
 *   String-based — hex I/O, callable directly from Ruyi.
 *   Raw pointer  — binary I/O for advanced / Buffer-bridge use.
 *
 * @author Ruyi Team
 * @date 2026-07-24
 */
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use aes_gcm::aead::AeadInPlace;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce, Tag};
use hmac::Mac;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};
use x25519_dalek::{PublicKey, StaticSecret};

// ── helpers ──────────────────────────────────────────────────

unsafe fn cstr_bytes(ptr: *const c_char) -> &'static [u8] {
    CStr::from_ptr(ptr).to_bytes()
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for i in (0..hex.len()).step_by(2) {
        let b = u8::from_str_radix(&hex[i..i + 2], 16).ok()?;
        bytes.push(b);
    }
    Some(bytes)
}

fn to_cstring(s: String) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

/// Failure value for the string-based API.
///
/// These symbols are declared to Ruyi as non-nullable `string`, and the
/// stdlib wrappers hand the result straight to user code, so a null
/// pointer would surface as a segfault on the next string operation.
/// An empty string is unambiguous: no digest, ciphertext or public key is
/// ever empty, so callers can test `result === ""` for failure.
fn cstring_error() -> *mut c_char {
    to_cstring(String::new())
}

// ============================================================
// String-based Hash API
// ============================================================

#[no_mangle]
pub extern "C" fn __crypto_sha256(data: *const c_char) -> *mut c_char {
    if data.is_null() {
        return cstring_error();
    }
    let input = unsafe { cstr_bytes(data) };
    let hash = Sha256::digest(input);
    to_cstring(hex_encode(&hash))
}

#[no_mangle]
pub extern "C" fn __crypto_sha512(data: *const c_char) -> *mut c_char {
    if data.is_null() {
        return cstring_error();
    }
    let input = unsafe { cstr_bytes(data) };
    let hash = Sha512::digest(input);
    to_cstring(hex_encode(&hash))
}

#[no_mangle]
pub extern "C" fn __crypto_sha1(data: *const c_char) -> *mut c_char {
    if data.is_null() {
        return cstring_error();
    }
    let input = unsafe { cstr_bytes(data) };
    let hash = Sha1::digest(input);
    to_cstring(hex_encode(&hash))
}

// ============================================================
// String-based HMAC API
// ============================================================

#[no_mangle]
pub extern "C" fn __crypto_hmac_sha256(key: *const c_char, data: *const c_char) -> *mut c_char {
    if key.is_null() || data.is_null() {
        return cstring_error();
    }
    let k = unsafe { cstr_bytes(key) };
    let d = unsafe { cstr_bytes(data) };

    let mut mac = <hmac::Hmac<Sha256> as hmac::Mac>::new_from_slice(k).unwrap_or_else(|_| {
        let hashed = Sha256::digest(k);
        <hmac::Hmac<Sha256> as hmac::Mac>::new_from_slice(&hashed).unwrap()
    });
    <hmac::Hmac<Sha256> as hmac::Mac>::update(&mut mac, d);
    let result = mac.finalize().into_bytes();
    to_cstring(hex_encode(&result))
}

// ============================================================
// Raw pointer HMAC API
// ============================================================
//
// PBKDF2 feeds each 32-byte digest back in as the next message, which the
// string API cannot carry: the digest is not valid UTF-8, so a string
// round-trip corrupts it. These take the bytes directly.

/// Read an array header's elements as bytes.
///
/// The crypto byte arrays (`zeroArray`, `stringToBytes`) are built with
/// `push(<int literal>)`, which monomorphizes the array on the *value*
/// type, so each element occupies an `i64` rather than a byte. Reinterpret
/// the element buffer accordingly.
unsafe fn array_bytes<'a>(header: *const u8) -> Option<&'a [i64]> {
    if header.is_null() {
        return None;
    }
    let len = *(header as *const i64);
    if len < 0 {
        return None;
    }
    let data = *(header.add(16) as *const i64) as *const i64;
    if data.is_null() {
        return Some(&[]);
    }
    Some(std::slice::from_raw_parts(data, len as usize))
}

/// Narrow an element slice to the bytes it represents.
fn pack(elements: &[i64]) -> Vec<u8> {
    elements.iter().map(|v| *v as u8).collect()
}

/// Widen `bytes` into an array's element buffer, truncating to `len`.
unsafe fn unpack(elements: *mut i64, len: usize, bytes: &[u8]) {
    for (i, b) in bytes.iter().take(len).enumerate() {
        *elements.add(i) = *b as i64;
    }
}

/// HMAC-SHA-256 over an `Array<byte>`, writing the 32-byte digest into
/// `out`.
///
/// # Safety
/// `key`, `data` and `out` must each point at a valid Ruyi array header,
/// and `out` must hold at least 32 elements. Returns 0 on success and -1
/// if any argument is malformed.
#[no_mangle]
pub extern "C" fn __crypto_hmac_sha256_raw(key: *const u8, data: *const u8, out: *mut u8) -> i32 {
    let (k, d, out_elems) = match (
        unsafe { array_bytes(key) },
        unsafe { array_bytes(data) },
        unsafe { array_bytes_len_and_data(out) },
    ) {
        (Some(k), Some(d), Some(o)) => (pack(k), pack(d), o),
        _ => return -1,
    };
    let mut mac = <hmac::Hmac<Sha256> as hmac::Mac>::new_from_slice(&k).unwrap_or_else(|_| {
        let hashed = Sha256::digest(&k);
        <hmac::Hmac<Sha256> as hmac::Mac>::new_from_slice(&hashed).unwrap()
    });
    <hmac::Hmac<Sha256> as hmac::Mac>::update(&mut mac, &d);
    let result = mac.finalize().into_bytes();
    unsafe { unpack(out_elems.0, out_elems.1, &result) };
    0
}

/// HMAC-SHA-512 over an `Array<byte>`, writing the 64-byte digest into
/// `out`.
///
/// # Safety
/// Same contract as [`__crypto_hmac_sha256_raw`], with a 64-byte output.
#[no_mangle]
pub extern "C" fn __crypto_hmac_sha512_raw(key: *const u8, data: *const u8, out: *mut u8) -> i32 {
    let (k, d, out_elems) = match (
        unsafe { array_bytes(key) },
        unsafe { array_bytes(data) },
        unsafe { array_bytes_len_and_data(out) },
    ) {
        (Some(k), Some(d), Some(o)) => (pack(k), pack(d), o),
        _ => return -1,
    };
    let mut mac = <hmac::Hmac<Sha512> as hmac::Mac>::new_from_slice(&k).unwrap_or_else(|_| {
        let hashed = Sha512::digest(&k);
        <hmac::Hmac<Sha512> as hmac::Mac>::new_from_slice(&hashed).unwrap()
    });
    <hmac::Hmac<Sha512> as hmac::Mac>::update(&mut mac, &d);
    let result = mac.finalize().into_bytes();
    unsafe { unpack(out_elems.0, out_elems.1, &result) };
    0
}

/// Writable view of an array header: element pointer and element count.
unsafe fn array_bytes_len_and_data(header: *const u8) -> Option<(*mut i64, usize)> {
    if header.is_null() {
        return None;
    }
    let len = *(header as *const i64);
    if len < 0 {
        return None;
    }
    let data = *(header.add(16) as *const i64) as *mut i64;
    if data.is_null() {
        return None;
    }
    Some((data, len as usize))
}

// ============================================================
// Hex-based AES-256-GCM API
// ============================================================

#[no_mangle]
pub extern "C" fn __crypto_aes_gcm_encrypt_hex(
    key_hex: *const c_char,
    nonce_hex: *const c_char,
    plain_hex: *const c_char,
) -> *mut c_char {
    if key_hex.is_null() || nonce_hex.is_null() || plain_hex.is_null() {
        return cstring_error();
    }
    let k = match hex_decode(unsafe { CStr::from_ptr(key_hex) }.to_str().unwrap_or("")) {
        Some(v) if v.len() == 32 => v,
        _ => return cstring_error(),
    };
    let n = match hex_decode(unsafe { CStr::from_ptr(nonce_hex) }.to_str().unwrap_or("")) {
        Some(v) if v.len() == 12 => v,
        _ => return cstring_error(),
    };
    let p = match hex_decode(unsafe { CStr::from_ptr(plain_hex) }.to_str().unwrap_or("")) {
        Some(v) => v,
        _ => return cstring_error(),
    };

    let cipher = Aes256Gcm::new_from_slice(&k).unwrap();
    let nonce = Nonce::from_slice(&n);
    let mut buf = p;
    match cipher.encrypt_in_place_detached(nonce, b"", &mut buf) {
        Ok(tag) => {
            let mut out = hex_encode(&buf);
            out.push_str(&hex_encode(tag.as_slice()));
            to_cstring(out)
        }
        Err(_) => cstring_error(),
    }
}

#[no_mangle]
pub extern "C" fn __crypto_aes_gcm_decrypt_hex(
    key_hex: *const c_char,
    nonce_hex: *const c_char,
    combined_hex: *const c_char,
) -> *mut c_char {
    if key_hex.is_null() || nonce_hex.is_null() || combined_hex.is_null() {
        return cstring_error();
    }
    let k = match hex_decode(unsafe { CStr::from_ptr(key_hex) }.to_str().unwrap_or("")) {
        Some(v) if v.len() == 32 => v,
        _ => return cstring_error(),
    };
    let n = match hex_decode(unsafe { CStr::from_ptr(nonce_hex) }.to_str().unwrap_or("")) {
        Some(v) if v.len() == 12 => v,
        _ => return cstring_error(),
    };
    let combined = hex_decode(
        unsafe { CStr::from_ptr(combined_hex) }
            .to_str()
            .unwrap_or(""),
    );
    let combined = match combined {
        Some(v) if v.len() >= 16 => v,
        _ => return cstring_error(),
    };

    let cipher_len = combined.len() - 16;
    let mut ct = combined[..cipher_len].to_vec();
    let t = Tag::from_slice(&combined[cipher_len..]);

    let c = Aes256Gcm::new_from_slice(&k).unwrap();
    let nonce = Nonce::from_slice(&n);
    match c.decrypt_in_place_detached(nonce, b"", &mut ct, t) {
        Ok(()) => to_cstring(hex_encode(&ct)),
        Err(_) => cstring_error(),
    }
}

// ============================================================
// Hex-based X25519 API
// ============================================================

#[no_mangle]
pub extern "C" fn __crypto_x25519_keypair_hex() -> *mut c_char {
    let secret = StaticSecret::random();
    let public = PublicKey::from(&secret);
    let mut out = hex_encode(public.as_bytes());
    out.push_str(&hex_encode(&secret.to_bytes()));
    to_cstring(out)
}

#[no_mangle]
pub extern "C" fn __crypto_x25519_dh_hex(
    priv_hex: *const c_char,
    peer_pub_hex: *const c_char,
) -> *mut c_char {
    if priv_hex.is_null() || peer_pub_hex.is_null() {
        return cstring_error();
    }
    let priv_bytes = match hex_decode(unsafe { CStr::from_ptr(priv_hex) }.to_str().unwrap_or("")) {
        Some(v) if v.len() == 32 => v,
        _ => return cstring_error(),
    };
    let pub_bytes = match hex_decode(
        unsafe { CStr::from_ptr(peer_pub_hex) }
            .to_str()
            .unwrap_or(""),
    ) {
        Some(v) if v.len() == 32 => v,
        _ => return cstring_error(),
    };

    let priv_arr: [u8; 32] = priv_bytes[..].try_into().unwrap();
    let pub_arr: [u8; 32] = pub_bytes[..].try_into().unwrap();
    let secret = StaticSecret::from(priv_arr);
    let peer = PublicKey::from(pub_arr);
    let shared = secret.diffie_hellman(&peer);
    to_cstring(hex_encode(shared.as_bytes()))
}

#[no_mangle]
pub extern "C" fn __crypto_x25519_pubkey_hex(priv_hex: *const c_char) -> *mut c_char {
    if priv_hex.is_null() {
        return cstring_error();
    }
    let priv_bytes = match hex_decode(unsafe { CStr::from_ptr(priv_hex) }.to_str().unwrap_or("")) {
        Some(v) if v.len() == 32 => v,
        _ => return cstring_error(),
    };
    let priv_arr: [u8; 32] = priv_bytes[..].try_into().unwrap();
    let secret = StaticSecret::from(priv_arr);
    let public = PublicKey::from(&secret);
    to_cstring(hex_encode(public.as_bytes()))
}

// ============================================================
// Raw pointer AES-256-GCM (advanced — needs byte-buffer bridge)
// ============================================================

#[no_mangle]
pub extern "C" fn __crypto_aes_gcm_encrypt_raw(
    key: *const u8,
    nonce: *const u8,
    plain: *const u8,
    plain_len: i64,
    cipher_out: *mut u8,
    tag_out: *mut u8,
) -> i32 {
    if key.is_null()
        || nonce.is_null()
        || plain.is_null()
        || cipher_out.is_null()
        || tag_out.is_null()
    {
        return -1;
    }
    let k = unsafe { std::slice::from_raw_parts(key, 32) };
    let n = unsafe { std::slice::from_raw_parts(nonce, 12) };
    let p = unsafe { std::slice::from_raw_parts(plain, plain_len as usize) };

    let cipher = Aes256Gcm::new_from_slice(k).unwrap();
    let nonce = Nonce::from_slice(n);
    let mut buf = p.to_vec();
    match cipher.encrypt_in_place_detached(nonce, b"", &mut buf) {
        Ok(tag) => {
            unsafe {
                std::ptr::copy_nonoverlapping(buf.as_ptr(), cipher_out, buf.len());
                std::ptr::copy_nonoverlapping(tag.as_slice().as_ptr(), tag_out, 16);
            }
            0
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn __crypto_aes_gcm_decrypt_raw(
    key: *const u8,
    nonce: *const u8,
    cipher: *const u8,
    cipher_len: i64,
    tag: *const u8,
    plain_out: *mut u8,
) -> i32 {
    if key.is_null() || nonce.is_null() || cipher.is_null() || plain_out.is_null() || tag.is_null()
    {
        return -1;
    }
    let k = unsafe { std::slice::from_raw_parts(key, 32) };
    let n = unsafe { std::slice::from_raw_parts(nonce, 12) };
    let ct = unsafe { std::slice::from_raw_parts(cipher, cipher_len as usize) };
    let t = Tag::from_slice(unsafe { std::slice::from_raw_parts(tag, 16) });

    let c = Aes256Gcm::new_from_slice(k).unwrap();
    let nonce = Nonce::from_slice(n);
    let mut buf = ct.to_vec();
    match c.decrypt_in_place_detached(nonce, b"", &mut buf, t) {
        Ok(()) => {
            unsafe { std::ptr::copy_nonoverlapping(buf.as_ptr(), plain_out, buf.len()) };
            0
        }
        Err(_) => 1,
    }
}

// ============================================================
// Raw pointer X25519 (advanced — needs byte-buffer bridge)
// ============================================================

#[no_mangle]
pub extern "C" fn __crypto_x25519_keypair_raw(pub_out: *mut u8, priv_out: *mut u8) {
    if pub_out.is_null() || priv_out.is_null() {
        return;
    }
    let secret = StaticSecret::random();
    let public = PublicKey::from(&secret);
    unsafe {
        std::ptr::copy_nonoverlapping(public.as_bytes().as_ptr(), pub_out, 32);
        std::ptr::copy_nonoverlapping(secret.to_bytes().as_ptr(), priv_out, 32);
    }
}

#[no_mangle]
pub extern "C" fn __crypto_x25519_dh_raw(
    priv_key: *const u8,
    peer_pub: *const u8,
    shared_out: *mut u8,
) {
    if priv_key.is_null() || peer_pub.is_null() || shared_out.is_null() {
        return;
    }
    let pk: [u8; 32] = unsafe { std::ptr::read(priv_key as *const [u8; 32]) };
    let pp: [u8; 32] = unsafe { std::ptr::read(peer_pub as *const [u8; 32]) };
    let secret = StaticSecret::from(pk);
    let peer = PublicKey::from(pp);
    let shared = secret.diffie_hellman(&peer);
    unsafe { std::ptr::copy_nonoverlapping(shared.as_bytes().as_ptr(), shared_out, 32) };
}

#[no_mangle]
pub extern "C" fn __crypto_x25519_pubkey_raw(priv_key: *const u8, pub_out: *mut u8) {
    if priv_key.is_null() || pub_out.is_null() {
        return;
    }
    let pk: [u8; 32] = unsafe { std::ptr::read(priv_key as *const [u8; 32]) };
    let secret = StaticSecret::from(pk);
    let public = PublicKey::from(&secret);
    unsafe { std::ptr::copy_nonoverlapping(public.as_bytes().as_ptr(), pub_out, 32) };
}
