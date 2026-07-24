/**
 * Compress FFI — gzip, zlib, and raw Deflate via flate2.
 *
 * String-based I/O: input/output are base64-encoded strings.
 * The base64 bridge avoids null-byte issues in compressed binary data
 * and follows the same pattern as crypto/hash_ffi.rs (hex bridge).
 *
 * @author Ruyi Team
 * @date 2026-07-24
 */

use std::ffi::CStr;
use std::io::Write;
use std::os::raw::c_char;

use flate2::write::{
    DeflateDecoder, DeflateEncoder, GzDecoder, GzEncoder, ZlibDecoder, ZlibEncoder,
};
use flate2::Compression;

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn b64enc(bytes: &[u8]) -> String {
    let mut s = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for ch in bytes.chunks(3) {
        let b0 = ch[0] as u32;
        let b1 = *ch.get(1).unwrap_or(&0) as u32;
        let b2 = *ch.get(2).unwrap_or(&0) as u32;
        let t = (b0 << 16) | (b1 << 8) | b2;
        s.push(B64[((t >> 18) & 63) as usize] as char);
        s.push(B64[((t >> 12) & 63) as usize] as char);
        s.push(if ch.len() > 1 { B64[((t >> 6) & 63) as usize] as char } else { '=' });
        s.push(if ch.len() > 2 { B64[(t & 63) as usize] as char } else { '=' });
    }
    s
}

fn b64dec(enc: &str) -> Option<Vec<u8>> {
    let s = enc.trim_end_matches('=');
    if s.is_empty() { return Some(Vec::new()); }
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    for ch in s.as_bytes().chunks(4) {
        let vals: Vec<u32> = ch.iter().filter_map(|&c| match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' => Some(62), b'/' => Some(63), _ => None,
        }).collect();
        if vals.is_empty() { continue; }
        let t = (vals[0] << 18) | (*vals.get(1).unwrap_or(&0) << 12) | (*vals.get(2).unwrap_or(&0) << 6) | *vals.get(3).unwrap_or(&0);
        out.push(((t >> 16) & 0xFF) as u8);
        if vals.len() > 2 { out.push(((t >> 8) & 0xFF) as u8); }
        if vals.len() > 3 { out.push((t & 0xFF) as u8); }
    }
    Some(out)
}

fn to_cs(s: String) -> *mut c_char {
    std::ffi::CString::new(s).map(|cs| cs.into_raw()).unwrap_or(std::ptr::null_mut())
}

// ── oneshot helpers ──────────────────────────────────────────

fn compress_gzip_inner(input: &[u8]) -> Option<Vec<u8>> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(input).ok()?;
    enc.finish().ok()
}

fn decompress_gzip_inner(input: &[u8]) -> Option<Vec<u8>> {
    let mut dec = GzDecoder::new(Vec::new());
    dec.write_all(input).ok()?;
    dec.finish().ok()
}

fn compress_zlib_inner(input: &[u8]) -> Option<Vec<u8>> {
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(input).ok()?;
    enc.finish().ok()
}

fn decompress_zlib_inner(input: &[u8]) -> Option<Vec<u8>> {
    let mut dec = ZlibDecoder::new(Vec::new());
    dec.write_all(input).ok()?;
    dec.finish().ok()
}

fn compress_deflate_inner(input: &[u8]) -> Option<Vec<u8>> {
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
    enc.write_all(input).ok()?;
    enc.finish().ok()
}

fn decompress_deflate_inner(input: &[u8]) -> Option<Vec<u8>> {
    let mut dec = DeflateDecoder::new(Vec::new());
    dec.write_all(input).ok()?;
    dec.finish().ok()
}

macro_rules! ffi_oneshot {
    ($export:ident, $inner:ident) => {
        #[no_mangle]
        pub extern "C" fn $export(b64: *const c_char) -> *mut c_char {
            if b64.is_null() { return std::ptr::null_mut(); }
            let input_str = match unsafe { CStr::from_ptr(b64) }.to_str() {
                Ok(s) => s, Err(_) => return std::ptr::null_mut(),
            };
            let input = match b64dec(input_str) {
                Some(v) => v, None => return std::ptr::null_mut(),
            };
            let out = match $inner(&input) {
                Some(v) => v, None => return std::ptr::null_mut(),
            };
            to_cs(b64enc(&out))
        }
    };
}

ffi_oneshot!(__compress_gzip, compress_gzip_inner);
ffi_oneshot!(__decompress_gzip, decompress_gzip_inner);
ffi_oneshot!(__compress_zlib, compress_zlib_inner);
ffi_oneshot!(__decompress_zlib, decompress_zlib_inner);
ffi_oneshot!(__compress_deflate, compress_deflate_inner);
ffi_oneshot!(__decompress_deflate, decompress_deflate_inner);

// ── streaming types ──────────────────────────────────────────

enum CStream {
    Gzip(GzEncoder<Vec<u8>>, usize),
    Zlib(ZlibEncoder<Vec<u8>>, usize),
    Deflate(DeflateEncoder<Vec<u8>>, usize),
}

enum DStream {
    Gzip(GzDecoder<Vec<u8>>, usize),
    Zlib(ZlibDecoder<Vec<u8>>, usize),
    Deflate(DeflateDecoder<Vec<u8>>, usize),
}

macro_rules! stream_write {
    ($enc:expr, $off:expr, $input:expr) => {{
        if $enc.write_all($input).is_err() || $enc.flush().is_err() {
            return std::ptr::null_mut();
        }
        let inner = $enc.get_ref();
        let new_bytes = inner[*$off..].to_vec();
        *$off = inner.len();
        new_bytes
    }};
}

macro_rules! stream_finish_impl {
    ($enc:expr, $off:expr) => {{
        match $enc.finish() {
            Ok(inner) => inner[$off..].to_vec(),
            Err(_) => Vec::new(),
        }
    }};
}

#[no_mangle]
pub extern "C" fn __compress_new(format: i64) -> *mut c_char {
    let cs = match format {
        0 => CStream::Gzip(GzEncoder::new(Vec::new(), Compression::default()), 0),
        1 => CStream::Zlib(ZlibEncoder::new(Vec::new(), Compression::default()), 0),
        2 => CStream::Deflate(DeflateEncoder::new(Vec::new(), Compression::default()), 0),
        _ => return std::ptr::null_mut(),
    };
    Box::into_raw(Box::new(cs)) as *mut c_char
}

#[no_mangle]
pub extern "C" fn __compress_write(handle: *mut c_char, b64: *const c_char) -> *mut c_char {
    if handle.is_null() || b64.is_null() { return std::ptr::null_mut(); }
    let input = match b64dec(unsafe { CStr::from_ptr(b64) }.to_str().unwrap_or("")) {
        Some(v) => v, None => return std::ptr::null_mut(),
    };
    let stream = unsafe { &mut *(handle as *mut CStream) };
    let new_bytes = match stream {
        CStream::Gzip(ref mut enc, ref mut off) => stream_write!(enc, off, &input),
        CStream::Zlib(ref mut enc, ref mut off) => stream_write!(enc, off, &input),
        CStream::Deflate(ref mut enc, ref mut off) => stream_write!(enc, off, &input),
    };
    to_cs(b64enc(&new_bytes))
}

#[no_mangle]
pub extern "C" fn __compress_finish(handle: *mut c_char) -> *mut c_char {
    if handle.is_null() { return std::ptr::null_mut(); }
    let stream = unsafe { Box::from_raw(handle as *mut CStream) };
    let new_bytes = match *stream {
        CStream::Gzip(enc, off) => stream_finish_impl!(enc, off),
        CStream::Zlib(enc, off) => stream_finish_impl!(enc, off),
        CStream::Deflate(enc, off) => stream_finish_impl!(enc, off),
    };
    to_cs(b64enc(&new_bytes))
}

#[no_mangle]
pub extern "C" fn __decompress_new(format: i64) -> *mut c_char {
    let ds = match format {
        0 => DStream::Gzip(GzDecoder::new(Vec::new()), 0),
        1 => DStream::Zlib(ZlibDecoder::new(Vec::new()), 0),
        2 => DStream::Deflate(DeflateDecoder::new(Vec::new()), 0),
        _ => return std::ptr::null_mut(),
    };
    Box::into_raw(Box::new(ds)) as *mut c_char
}

#[no_mangle]
pub extern "C" fn __decompress_write(handle: *mut c_char, b64: *const c_char) -> *mut c_char {
    if handle.is_null() || b64.is_null() { return std::ptr::null_mut(); }
    let input = match b64dec(unsafe { CStr::from_ptr(b64) }.to_str().unwrap_or("")) {
        Some(v) => v, None => return std::ptr::null_mut(),
    };
    let stream = unsafe { &mut *(handle as *mut DStream) };
    let new_bytes = match stream {
        DStream::Gzip(ref mut dec, ref mut off) => stream_write!(dec, off, &input),
        DStream::Zlib(ref mut dec, ref mut off) => stream_write!(dec, off, &input),
        DStream::Deflate(ref mut dec, ref mut off) => stream_write!(dec, off, &input),
    };
    to_cs(b64enc(&new_bytes))
}

#[no_mangle]
pub extern "C" fn __decompress_finish(handle: *mut c_char) -> *mut c_char {
    if handle.is_null() { return std::ptr::null_mut(); }
    let stream = unsafe { Box::from_raw(handle as *mut DStream) };
    let new_bytes = match *stream {
        DStream::Gzip(dec, off) => stream_finish_impl!(dec, off),
        DStream::Zlib(dec, off) => stream_finish_impl!(dec, off),
        DStream::Deflate(dec, off) => stream_finish_impl!(dec, off),
    };
    to_cs(b64enc(&new_bytes))
}

// ── tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test] fn test_b64_roundtrip() {
        let d = b"Hello, Ruyi!\x00\xFF\x80";
        assert_eq!(b64dec(&b64enc(d)).unwrap(), d);
    }

    #[test] fn test_gzip() {
        let i = b"Hello, Ruyi! Gzip roundtrip test.";
        let c = __compress_gzip(CString::new(b64enc(i)).unwrap().as_ptr());
        let d = __decompress_gzip(c);
        assert_eq!(b64dec(unsafe { CStr::from_ptr(d) }.to_str().unwrap()).unwrap(), i);
    }

    #[test] fn test_zlib() {
        let i = b"Zlib test for Ruyi.";
        let c = __compress_zlib(CString::new(b64enc(i)).unwrap().as_ptr());
        let d = __decompress_zlib(c);
        assert_eq!(b64dec(unsafe { CStr::from_ptr(d) }.to_str().unwrap()).unwrap(), i);
    }

    #[test] fn test_deflate() {
        let i = b"Raw deflate for WS.";
        let c = __compress_deflate(CString::new(b64enc(i)).unwrap().as_ptr());
        let d = __decompress_deflate(c);
        assert_eq!(b64dec(unsafe { CStr::from_ptr(d) }.to_str().unwrap()).unwrap(), i);
    }

    #[test] fn test_null() {
        assert!(__compress_gzip(std::ptr::null()).is_null());
    }

    #[test] fn test_streaming_gzip() {
        let parts: [&[u8]; 3] = [b"Hello, ", b"streaming ", b"gzip!"];
        let h = __compress_new(0);
        assert!(!h.is_null());
        let mut all_compressed = Vec::new();
        for p in &parts {
            let b64 = CString::new(b64enc(p)).unwrap();
            let out = __compress_write(h, b64.as_ptr());
            assert!(!out.is_null());
            all_compressed.extend_from_slice(b64dec(unsafe {
                CStr::from_ptr(out)
            }.to_str().unwrap()).unwrap().as_slice());
        }
        let final_out = __compress_finish(h);
        all_compressed.extend_from_slice(b64dec(unsafe {
            CStr::from_ptr(final_out)
        }.to_str().unwrap()).unwrap().as_slice());
        let b64_all = CString::new(b64enc(&all_compressed)).unwrap();
        let dec = __decompress_gzip(b64_all.as_ptr());
        let combined: Vec<u8> = parts.concat();
        assert_eq!(
            b64dec(unsafe { CStr::from_ptr(dec) }.to_str().unwrap()).unwrap(),
            combined
        );
    }
}
