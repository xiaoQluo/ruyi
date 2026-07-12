/**
 * Integration tests for the FFI bindings exposed from
 * `crates/ruyi_runtime/src/fmt_ffi.rs`.
 *
 * `ruyi_string_replace_all` writes a transformed copy of an input buffer
 * into a caller-supplied output buffer. The function is exercised directly
 * here so that linker errors surface as concrete test failures before the
 * `stdlib/fmt.ry` consumer is wired up.
 *
 * Each test arranges an output buffer, calls the symbol with bounded
 * lengths, then reads back the returned `usize` count of written bytes.
 *
 * @author Ruyi Team
 * @date 2026-07-11
 */
use ruyi_runtime::fmt_ffi::ruyi_string_replace_all;

/// `ruyi_string_replace_all("a.b.c", ".", "/")` MUST yield `"a/b/c"`.
/// The replacement target is longer than the pattern, exercising the
/// write-ahead branch.
#[test]
fn test_ruyi_string_replace_all_basic() {
    let input: &[u8] = b"a.b.c";
    let from: &[u8] = b".";
    let to: &[u8] = b"/";
    // Worst case: every byte is replaced with the same length, so the
    // output is at most `input.len()` bytes.
    let mut out = [0u8; 16];
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
    assert_eq!(written, 5, "expected 5 bytes written, got {}", written);
    assert_eq!(&out[..written], b"a/b/c");
}

/// `ruyi_string_replace_all("hello", "x", "y")` MUST return the input
/// unchanged when `from` is not present. The output buffer must still be
/// readable as a UTF-8 slice of the input length.
#[test]
fn test_ruyi_string_replace_all_no_match() {
    let input: &[u8] = b"hello";
    let from: &[u8] = b"x";
    let to: &[u8] = b"y";
    let mut out = [0u8; 16];
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
    assert_eq!(written, input.len(), "expected no-op, got {}", written);
    assert_eq!(&out[..written], b"hello");
}

/// `ruyi_string_replace_all("abc", "b", "")` MUST yield `"ac"`. The
/// replacement target has length zero, exercising the delete branch.
#[test]
fn test_ruyi_string_replace_all_empty_to() {
    let input: &[u8] = b"abc";
    let from: &[u8] = b"b";
    let to: &[u8] = b"";
    let mut out = [0u8; 16];
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
    assert_eq!(written, 2, "expected 2 bytes written, got {}", written);
    assert_eq!(&out[..written], b"ac");
}
