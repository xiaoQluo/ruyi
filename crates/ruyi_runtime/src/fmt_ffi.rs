/**
 * C FFI for `stdlib/fmt.ry` format helpers.
 *
 * Currently exposes a single `#[no_mangle] extern "C"` symbol —
 * `ruyi_string_replace_all` — which performs a bounded, in-place buffer
 * substitution used by `fmt.format` to expand `{}` placeholders.
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
pub unsafe extern "C" fn ruyi_string_replace_all(
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

#[cfg(test)]
mod tests {
    use super::*;

    fn replace(input: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; input.len() * (to.len().max(1) + 1) + 8];
        let written = unsafe {
            ruyi_string_replace_all(
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
            ruyi_string_replace_all(
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
