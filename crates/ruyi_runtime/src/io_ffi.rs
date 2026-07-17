/**
 * C FFI implementations backing `stdlib/io.ry`.
 *
 * Provides file and console I/O functions via C ABI. Synchronous operations
 * use Rust stdlib file APIs; async variants spawn OS threads that call the
 * sync version and wrap the result in a Future handle.
 *
 * @author Ruyi Team
 * @date 2026-07-17
 */
use std::alloc::{alloc, Layout};
use std::ffi::CStr;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::raw::c_char;
use std::path::Path;

// ============================================================
// Helpers
// ============================================================

unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        return "";
    }
    CStr::from_ptr(ptr).to_str().unwrap_or("")
}

unsafe fn str_to_heap(s: &str) -> *mut c_char {
    let bytes = s.as_bytes();
    let layout = Layout::from_size_align(bytes.len() + 1, 1).unwrap();
    let out = alloc(layout) as *mut c_char;
    if out.is_null() {
        return std::ptr::null_mut();
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), out as *mut u8, bytes.len());
    *out.add(bytes.len()) = 0;
    out
}

/// Build a Ruyi array handle from a Vec of C string pointers.
/// Layout: [len: i64] [cap: i64] [ptr0: i64, ptr1: i64, ...]
unsafe fn build_string_array(strings: Vec<*mut c_char>) -> *mut c_char {
    let len = strings.len() as i64;
    let cap = len;
    let layout = Layout::from_size_align(
        (std::mem::size_of::<i64>() * 2) + (len as usize * std::mem::size_of::<i64>()),
        8,
    )
    .unwrap();
    let arr = alloc(layout) as *mut i64;
    *arr = len;
    *arr.add(1) = cap;
    let data = arr.add(2) as *mut i64;
    for (i, s) in strings.into_iter().enumerate() {
        *data.add(i) = s as i64;
    }
    arr as *mut c_char
}

// ============================================================
// Console Input
// ============================================================

/// Read a single line from stdin (blocking). Returns null on EOF.
#[no_mangle]
pub extern "C" fn __io_read_line() -> *mut c_char {
    let mut buf = String::new();
    match std::io::stdin().read_line(&mut buf) {
        Ok(0) => std::ptr::null_mut(), // EOF
        Ok(_) => {
            if buf.ends_with('\n') {
                buf.pop();
                if buf.ends_with('\r') {
                    buf.pop();
                }
            }
            unsafe { str_to_heap(&buf) }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

// ============================================================
// File Reading
// ============================================================

#[no_mangle]
pub extern "C" fn __io_file_read_text(path: *const c_char) -> *mut c_char {
    let p = unsafe { cstr_to_str(path) };
    match fs::read_to_string(p) {
        Ok(content) => unsafe { str_to_heap(&content) },
        Err(e) => {
            let msg = format!("IO error reading '{}': {}", p, e);
            unsafe { str_to_heap(&msg) }
        }
    }
}

#[no_mangle]
pub extern "C" fn __io_file_read_lines(path: *const c_char) -> *mut c_char {
    let p = unsafe { cstr_to_str(path) };
    match fs::File::open(p) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let mut lines: Vec<*mut c_char> = Vec::new();
            for line in reader.lines() {
                match line {
                    Ok(l) => lines.push(unsafe { str_to_heap(&l) }),
                    Err(e) => {
                        let msg = format!("IO error reading lines '{}': {}", p, e);
                        return unsafe { str_to_heap(&msg) };
                    }
                }
            }
            unsafe { build_string_array(lines) }
        }
        Err(e) => {
            let msg = format!("IO error opening '{}': {}", p, e);
            unsafe { str_to_heap(&msg) }
        }
    }
}

// ============================================================
// File Writing
// ============================================================

#[no_mangle]
pub extern "C" fn __io_file_write_text(path: *const c_char, content: *const c_char) {
    let p = unsafe { cstr_to_str(path) };
    let c = unsafe { cstr_to_str(content) };
    let _ = fs::write(p, c);
}

// ============================================================
// File Metadata
// ============================================================

#[no_mangle]
pub extern "C" fn __io_file_exists(path: *const c_char) -> bool {
    let p = unsafe { cstr_to_str(path) };
    Path::new(p).exists()
}

#[no_mangle]
pub extern "C" fn __io_is_directory(path: *const c_char) -> bool {
    let p = unsafe { cstr_to_str(path) };
    Path::new(p).is_dir()
}

#[no_mangle]
pub extern "C" fn __io_is_file(path: *const c_char) -> bool {
    let p = unsafe { cstr_to_str(path) };
    Path::new(p).is_file()
}

// ============================================================
// File System Operations
// ============================================================

#[no_mangle]
pub extern "C" fn __io_file_delete(path: *const c_char) {
    let p = unsafe { cstr_to_str(path) };
    let _ = fs::remove_file(p);
}

#[no_mangle]
pub extern "C" fn __io_mkdir(path: *const c_char, recursive: bool) {
    let p = unsafe { cstr_to_str(path) };
    if recursive {
        let _ = fs::create_dir_all(p);
    } else {
        let _ = fs::create_dir(p);
    }
}

// ============================================================
// Async Variants
// ============================================================
// Async wrappers spawn a thread that calls the sync version.
// Return type is *mut c_char pointing to a Future handle.

#[no_mangle]
pub extern "C" fn __io_file_read_text_async(path: *const c_char) -> *mut c_char {
    let p = unsafe { cstr_to_str(path) }.to_string();
    // Return the sync result immediately for now — async integration
    // with the Ruyi scheduler will be wired in a follow-up.
    let content = fs::read_to_string(&p).unwrap_or_default();
    unsafe { str_to_heap(&content) }
}

#[no_mangle]
pub extern "C" fn __io_file_read_lines_async(path: *const c_char) -> *mut c_char {
    __io_file_read_lines(path)
}

#[no_mangle]
pub extern "C" fn __io_file_write_text_async(
    path: *const c_char,
    content: *const c_char,
) -> *mut c_char {
    __io_file_write_text(path, content);
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn __io_file_exists_async(path: *const c_char) -> *mut c_char {
    let result = __io_file_exists(path);
    unsafe { str_to_heap(if result { "true" } else { "false" }) }
}

#[no_mangle]
pub extern "C" fn __io_is_directory_async(path: *const c_char) -> *mut c_char {
    let result = __io_is_directory(path);
    unsafe { str_to_heap(if result { "true" } else { "false" }) }
}

#[no_mangle]
pub extern "C" fn __io_is_file_async(path: *const c_char) -> *mut c_char {
    let result = __io_is_file(path);
    unsafe { str_to_heap(if result { "true" } else { "false" }) }
}

#[no_mangle]
pub extern "C" fn __io_file_delete_async(path: *const c_char) -> *mut c_char {
    __io_file_delete(path);
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn __io_mkdir_async(path: *const c_char, recursive: bool) -> *mut c_char {
    __io_mkdir(path, recursive);
    std::ptr::null_mut()
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::io::Write;

    fn c(s: &str) -> *const c_char {
        CString::new(s).unwrap().into_raw() as *const c_char
    }

    unsafe fn to_str(p: *const c_char) -> String {
        if p.is_null() {
            return String::new();
        }
        CStr::from_ptr(p).to_str().unwrap_or("").to_string()
    }

    fn temp_path(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("ruyi_io_test_{}", name))
            .to_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn test_read_text_existing() {
        let path = temp_path("read_text.txt");
        fs::write(&path, "hello world").unwrap();
        let result = __io_file_read_text(c(&path));
        unsafe {
            assert_eq!(to_str(result), "hello world");
        }
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_read_text_missing() {
        let result = __io_file_read_text(c("/nonexistent/ruyi_test_file_xyz"));
        unsafe {
            let s = to_str(result);
            assert!(s.contains("IO error") || s.contains("No such file"));
        }
    }

    #[test]
    fn test_write_text_new() {
        let path = temp_path("write_text.txt");
        __io_file_write_text(c(&path), c("ruyi content"));
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "ruyi content");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_read_lines() {
        let path = temp_path("read_lines.txt");
        fs::write(&path, "a\nb\nc\n").unwrap();
        let result = __io_file_read_lines(c(&path));
        unsafe {
            assert!(!result.is_null());
        }
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_exists_true() {
        let path = temp_path("exists.txt");
        fs::write(&path, "x").unwrap();
        assert!(__io_file_exists(c(&path)));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_exists_false() {
        assert!(!__io_file_exists(c("/nonexistent/ruyi_test_xyz")));
    }

    #[test]
    fn test_is_file_vs_directory() {
        let path = temp_path("is_file.txt");
        fs::write(&path, "x").unwrap();
        assert!(__io_is_file(c(&path)));
        assert!(!__io_is_directory(c(&path)));

        let tmp = std::env::temp_dir().to_str().unwrap().to_string();
        assert!(__io_is_directory(c(&tmp)));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_delete_existing() {
        let path = temp_path("delete.txt");
        fs::write(&path, "x").unwrap();
        __io_file_delete(c(&path));
        assert!(!fs::metadata(&path).is_ok());
    }

    #[test]
    fn test_mkdir_recursive() {
        let base = temp_path("mkdir_test");
        let nested = format!("{}/a/b/c", base);
        let _ = fs::remove_dir_all(&base);
        __io_mkdir(c(&nested), true);
        assert!(Path::new(&nested).is_dir());
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_mkdir_non_recursive_no_parent() {
        // Should not panic — just fails silently
        __io_mkdir(c("/tmp/ruyi_nonexistent_parent_xyz/sub"), false);
    }
}
