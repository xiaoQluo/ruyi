/**
 * C FFI for `stdlib/fmt.ry` format helpers.
 *
 * Currently exposes a single `#[no_mangle] extern "C"` symbol —
 * `__string_replace_all` — which performs a bounded, in-place buffer
 * substitution used by `fmt.format` to expand `{}` placeholders.
 *
 * v0.5.9 / R3: this function was previously named `__string_replace_all`.
 * The rename unifies the canonical `__string_replace_all` symbol with the
 * bounded-buffer design; the older 3-arg variant (now in `builtins.rs`
 * under `__string_replace_all_legacy`) is preserved for source
 * compatibility.
 *
 * The caller supplies a pre-allocated output buffer via `out` / `out_cap`.
 * The function writes as many bytes as fit and returns the number of
 * bytes written. When `from` is empty the function short-circuits to 0 to
 * avoid unbounded looping (matching the stdlib convention that an empty
 * pattern never matches).
 *
 * The FFI deliberately avoids regex or external dependencies; the
 * substitution is a single linear pass over the input bytes.
 *
 * @author Ruyi Team
 * @date 2026-07-11
 */
/// Replace all occurrences of `from` with `to` in `s`, writing the result
/// into the caller-supplied output buffer.
///
/// # Safety
///
/// All four length-prefixed pointers (`s`, `from`, `to`, `out`) MUST be
/// non-null when their corresponding length is non-zero, and the pointed-to
/// memory MUST remain live and exclusively accessible for `s_len`,
/// `from_len`, `to_len`, and `out_cap` bytes respectively for the entire
/// duration of the call. The caller is responsible for sizing `out_cap`
/// to hold the worst-case expansion; the function truncates writes
/// silently rather than aborting on overflow.
///
/// # Arguments
///
/// * `s` — pointer to the source byte slice (need not be UTF-8).
/// * `s_len` — length in bytes of the source slice.
/// * `from` — pointer to the pattern byte slice (need not be UTF-8).
/// * `from_len` — length in bytes of the pattern slice. When `0` the
///   function returns `0` immediately without touching `out`.
/// * `to` — pointer to the replacement byte slice (need not be UTF-8).
/// * `to_len` — length in bytes of the replacement slice.
/// * `out` — pointer to a writable buffer of at least `out_cap` bytes.
/// * `out_cap` — capacity of `out` in bytes.
///
/// # Returns
///
/// Number of bytes written into `out`. When the buffer is too small for
/// the unbounded expansion, the function truncates the write to `out_cap`
/// (no allocation, no abort). Callers who care about lossless output
/// MUST pre-size the buffer to `s_len + ceil((s_len / from_len) * to_len)`
/// — comfortably larger than the worst case.
#[no_mangle]
pub unsafe extern "C" fn __string_replace_all(
    s: *const u8,
    s_len: usize,
    from: *const u8,
    from_len: usize,
    to: *const u8,
    to_len: usize,
    out: *mut u8,
    out_cap: usize,
) -> usize {
    if from_len == 0 {
        return 0;
    }
    // SAFETY: every length-prefixed pointer below is paired with its
    // exact length. Callers are documented to keep these pointers live
    // for the duration of the call.
    let s = unsafe { std::slice::from_raw_parts(s, s_len) };
    let from = unsafe { std::slice::from_raw_parts(from, from_len) };
    let to = unsafe { std::slice::from_raw_parts(to, to_len) };
    let out = unsafe { std::slice::from_raw_parts_mut(out, out_cap) };
    let mut written = 0usize;
    let mut i = 0usize;
    while i < s_len {
        if i + from_len <= s_len && s[i..i + from_len] == *from {
            for &b in to {
                if written < out_cap {
                    out[written] = b;
                    written += 1;
                }
            }
            i += from_len;
        } else {
            if written < out_cap {
                out[written] = s[i];
                written += 1;
            }
            i += 1;
        }
    }
    written
}

/// Sentinel values stored in the second field of a dyn element to
/// discriminate primitive kinds (string payloads use the field directly
/// as a `*mut i8` pointer, which is never a small integer).
const DYN_TAG_INT: i64 = 1;
const DYN_TAG_FLOAT: i64 = 2;
const DYN_TAG_BOOL: i64 = 3;

/// One-shot `format()` for variadic `dyn` arguments.
///
/// Receives the null-terminated `template` and the Ruyi `Array<dyn>`
/// handle `args`. The Array layout is `[len: i64][cap: i64][data_ptr:
/// *mut i8]`; each element slot is one 8-byte word holding a pointer to
/// a heap-boxed 16-byte dyn struct `{ i64 value, ptr data }` (the Array
/// box protocol — must match codegen `build_box_struct_to_word`). Performs every `{}` substitution in a
/// single Rust-side pass and returns a freshly allocated null-terminated
/// result string.
///
/// This avoids the Ruyi-side `for arg in args { args[i].toString() }`
/// loop, which currently mis-indexes variadic dyn arguments (the same
/// value is returned for every index — see v0.6.0-fix-examples B1).
///
/// `args` may be null when the caller has zero variadic arguments. The
/// returned pointer must be considered owned by the caller; the existing
/// convention for `__string_*` helpers is that these allocations are
/// leaked until the GC tracks them (see NOTE on `__string_*` in
/// `builtins.rs`).
///
/// # Safety
///
/// * `template` must be a valid null-terminated string when non-null.
/// * `args` must point to a valid Ruyi Array header when non-null.
/// * Both buffers must remain live for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn __fmt_format_dyn(template: *const i8, args: *mut i8) -> *mut i8 {
    unsafe {
        // Empty / null template: return empty string without touching args.
        if template.is_null() {
            return empty_alloc_string();
        }
        let template_str = match std::ffi::CStr::from_ptr(template).to_str() {
            Ok(s) => s,
            Err(_) => return empty_alloc_string(),
        };

        // Pull `len` and `data_ptr` from the Ruyi Array header.
        // Header layout: [len: i64][cap: i64][data_ptr: i64].
        // `array_data_ptr` from builtins.rs centralises the sanity
        // checks (null header, addresses below 0x1000, etc.).
        let (args_count, args_data) = if args.is_null() {
            (0usize, std::ptr::null_mut::<u8>())
        } else {
            let len_ptr = args as *const i64;
            let len_raw = std::ptr::read_unaligned(len_ptr);
            let len = if len_raw < 0 { 0 } else { len_raw as usize };
            // data_ptr sits at offset 16.
            let data_raw = std::ptr::read_unaligned(args.add(16) as *const i64);
            let data = if (data_raw as usize) < 0x1000 {
                std::ptr::null_mut::<u8>()
            } else {
                data_raw as *mut u8
            };
            (len, data)
        };

        // Iteratively expand "{}" in the template. Each step consumes the
        // next dyn element, picks the corresponding `*_to_string` helper
        // via the sentinel in the second slot, and substitutes the
        // result back into the working string. This keeps allocations
        // bounded (single in-place rewrite per step) and avoids the
        // Ruyi-side indexing bug.
        let mut current = template_str.to_string();
        for i in 0..args_count {
            if args_data.is_null() {
                // Defensive: empty data buffer even though len > 0.
                break;
            }
            let elem_ptr = args_data.add(i * 8);
            // Each slot is one 8-byte word holding the pointer to the
            // heap-boxed dyn struct; unbox before decoding. Words below
            // 0x1000 are corruption guards, same as the header checks.
            let box_raw = std::ptr::read_unaligned(elem_ptr as *const i64);
            if (box_raw as usize) < 0x1000 {
                break;
            }
            let box_ptr = box_raw as *mut u8;
            // dyn struct layout (must match codegen):
            //   field 0 (offset 0, i64): for primitives = the raw value,
            //                            for strings = 0 (unused).
            //   field 1 (offset 8, ptr): for primitives = a small
            //                            sentinel (DYN_TAG_*); for
            //                            strings = the *mut i8 payload.
            let tag = std::ptr::read_unaligned(box_ptr as *const i64);
            let data = std::ptr::read_unaligned(box_ptr.add(8) as *const i64);
            let replacement: String = match data {
                x if x == DYN_TAG_INT => tag.to_string(),
                x if x == DYN_TAG_FLOAT => {
                    let f = f64::from_bits(tag as u64);
                    format!("{}", f)
                }
                x if x == DYN_TAG_BOOL => {
                    if tag != 0 {
                        "true".to_string()
                    } else {
                        "false".to_string()
                    }
                }
                _ => {
                    let s_ptr = data as *const i8;
                    if s_ptr.is_null() {
                        String::new()
                    } else {
                        std::ffi::CStr::from_ptr(s_ptr)
                            .to_string_lossy()
                            .into_owned()
                    }
                }
            };
            // Replace the FIRST remaining "{}" so a user template that
            // contains more placeholders than args leaves the surplus
            // untouched — this matches the documented stdlib contract.
            if let Some(pos) = current.find("{}") {
                let mut buf = String::with_capacity(current.len() + replacement.len());
                buf.push_str(&current[..pos]);
                buf.push_str(&replacement);
                buf.push_str(&current[pos + 2..]);
                current = buf;
            } else {
                break;
            }
        }

        alloc_c_string(&current)
    }
}

/// Helper: allocate a fresh null-terminated C string for `s`.
unsafe fn alloc_c_string(s: &str) -> *mut i8 {
    let bytes = s.as_bytes();
    let layout = std::alloc::Layout::from_size_align(bytes.len() + 1, 1).unwrap();
    let out = std::alloc::alloc(layout) as *mut i8;
    if out.is_null() {
        return std::ptr::null_mut();
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), out as *mut u8, bytes.len());
    *out.add(bytes.len()) = 0;
    out
}

/// Helper: allocate a fresh empty null-terminated C string.
unsafe fn empty_alloc_string() -> *mut i8 {
    alloc_c_string("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{alloc, Layout};

    /// Build a Ruyi `Array<dyn>` header under the box protocol:
    /// [len: i64][cap: i64][data_ptr: i64], one 8-byte word per element
    /// where each word points to a heap-boxed 16-byte `{i64 tag, *mut i8
    /// data}` Dynamic struct.
    unsafe fn make_dyn_array(elems: &[(i64, i64)]) -> (*mut i8, Vec<*mut i8>) {
        let mut boxes = Vec::new();
        let data = alloc(Layout::from_size_align(elems.len() * 8, 8).unwrap()) as *mut i64;
        for (i, (tag, payload)) in elems.iter().enumerate() {
            let cell = alloc(Layout::from_size_align(16, 8).unwrap()) as *mut i64;
            *cell = *tag;
            *cell.add(1) = *payload;
            boxes.push(cell as *mut i8);
            *data.add(i) = cell as i64;
        }
        let header = alloc(Layout::from_size_align(24, 8).unwrap()) as *mut i64;
        *header = elems.len() as i64;
        *header.add(1) = (elems.len() * 8) as i64;
        *header.add(2) = data as i64;
        (header as *mut i8, boxes)
    }

    #[test]
    fn test_format_dyn_reads_boxed_elements() {
        let s = c"Ruyi";
        let (arr, _boxes) = unsafe {
            make_dyn_array(&[
                (0, s.as_ptr() as i64), // string: tag 0, data = char*
                (42, DYN_TAG_INT),      // int: tag = value, data = DYN_TAG_INT
            ])
        };
        let out = unsafe { __fmt_format_dyn(c"<{}> <{}>".as_ptr(), arr) };
        assert!(!out.is_null());
        let rendered = unsafe { std::ffi::CStr::from_ptr(out) };
        assert_eq!(rendered.to_bytes(), b"<Ruyi> <42>");
    }

    fn replace(input: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; input.len() * (to.len().max(1) + 1) + 8];
        let written = unsafe {
            __string_replace_all(
                input.as_ptr(),
                input.len(),
                from.as_ptr(),
                from.len(),
                to.as_ptr(),
                to.len(),
                out.as_mut_ptr(),
                out.len(),
            )
        };
        out.truncate(written);
        out
    }

    #[test]
    fn unit_basic_replace_longer_target() {
        assert_eq!(replace(b"a.b.c", b".", b"/"), b"a/b/c".to_vec());
    }

    #[test]
    fn unit_no_match_returns_input() {
        assert_eq!(replace(b"hello", b"x", b"y"), b"hello".to_vec());
    }

    #[test]
    fn unit_empty_pattern_returns_zero() {
        let mut out = [0u8; 16];
        let written = unsafe {
            __string_replace_all(
                b"hello".as_ptr(),
                5,
                b"".as_ptr(),
                0,
                b"x".as_ptr(),
                1,
                out.as_mut_ptr(),
                out.len(),
            )
        };
        assert_eq!(written, 0);
    }

    #[test]
    fn unit_replace_shorter_target_truncates_input() {
        assert_eq!(replace(b"abc", b"b", b""), b"ac".to_vec());
    }
}
