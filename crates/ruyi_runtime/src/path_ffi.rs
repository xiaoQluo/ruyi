/**
 * C FFI implementations backing `stdlib/path.ry`.
 *
 * Provides path manipulation functions via C ABI for use by Ruyi standard library.
 * All functions operate on null-terminated UTF-8 strings. Returned strings are
 * allocated via `ruyi_alloc` and freed by the Ruyi GC.
 *
 * @author Ruyi Team
 * @date 2026-07-17
 */
use std::alloc::{alloc, Layout};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::Path;

// ============================================================
// Helpers
// ============================================================

/// Convert a C string pointer to a Rust &str. Returns "" for null pointers.
unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        return "";
    }
    CStr::from_ptr(ptr).to_str().unwrap_or("")
}

/// Allocate a new null-terminated C string from a Rust &str.
/// Uses the system allocator (malloc) — consistent with `ruyi_alloc` / `__string_*` pattern.
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

/// Read the array length from an opaque array handle.
/// The array layout is: [length: i64] [capacity: i64] [elements: *const i8...]
unsafe fn array_len(arr: *mut c_char) -> i64 {
    if arr.is_null() {
        return 0;
    }
    *(arr as *const i64)
}

/// Read the element pointer at the given index from an array handle.
unsafe fn array_get(arr: *mut c_char, index: i64) -> *const c_char {
    let data = arr.add(std::mem::size_of::<i64>() * 2) as *const i64;
    *data.add(index as usize) as *const c_char
}

// ============================================================
// Path FFI Functions
// ============================================================

/// Join an array of path segments using the platform separator `/`.
///
/// # Safety
/// `segments` must be a valid array handle or null.
#[no_mangle]
pub extern "C" fn __path_join(segments: *mut c_char) -> *mut c_char {
    unsafe {
        let len = array_len(segments);
        if len == 0 {
            return str_to_heap("");
        }

        // First pass: collect segments and compute total length
        let mut parts: Vec<&str> = Vec::with_capacity(len as usize);
        let mut total_len: usize = 0;
        for i in 0..len {
            let elem_ptr = array_get(segments, i);
            let s = cstr_to_str(elem_ptr);
            if s.is_empty() {
                continue;
            }
            parts.push(s);
            total_len += s.len();
        }
        if parts.is_empty() {
            return str_to_heap("");
        }

        // Account for separators between segments
        total_len += parts.len().saturating_sub(1);

        // Second pass: build result
        let layout = Layout::from_size_align(total_len + 1, 1).unwrap();
        let out = alloc(layout) as *mut c_char;
        let mut pos: usize = 0;
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                *out.add(pos) = b'/' as c_char;
                pos += 1;
            }
            let bytes = part.as_bytes();
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), out.add(pos) as *mut u8, bytes.len());
            pos += bytes.len();
        }
        *out.add(pos) = 0;
        out
    }
}

/// Return the last component (filename) of a path.
#[no_mangle]
pub extern "C" fn __path_basename(path: *const c_char) -> *mut c_char {
    let s = unsafe { cstr_to_str(path) };
    let base = Path::new(s)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("");
    unsafe { str_to_heap(base) }
}

/// Return the directory portion of a path (everything before the last separator).
#[no_mangle]
pub extern "C" fn __path_dirname(path: *const c_char) -> *mut c_char {
    let s = unsafe { cstr_to_str(path) };
    let dir = Path::new(s).parent().and_then(|p| p.to_str()).unwrap_or("");
    // Preserve root "/" — Path::new("/foo").parent() returns Some("/")
    if dir.is_empty() && s.starts_with('/') {
        unsafe { str_to_heap("/") }
    } else {
        unsafe { str_to_heap(dir) }
    }
}

/// Return the file extension including the leading dot, or empty string if none.
/// For "file.tar.gz", returns ".gz" (last extension only, matching stdlib expectation).
#[no_mangle]
pub extern "C" fn __path_extname(path: *const c_char) -> *mut c_char {
    let s = unsafe { cstr_to_str(path) };
    let ext = Path::new(s)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if ext.is_empty() {
        unsafe { str_to_heap("") }
    } else {
        let dotted = format!(".{}", ext);
        unsafe { str_to_heap(&dotted) }
    }
}

/// Return true if the path is absolute (starts with `/` on Unix).
#[no_mangle]
pub extern "C" fn __path_is_absolute(path: *const c_char) -> bool {
    let s = unsafe { cstr_to_str(path) };
    Path::new(s).is_absolute()
}

/// Normalize a path by resolving `.` and `..` components and removing redundant separators.
#[no_mangle]
pub extern "C" fn __path_normalize(path: *const c_char) -> *mut c_char {
    let s = unsafe { cstr_to_str(path) };
    let p = Path::new(s);
    let mut components: Vec<&str> = Vec::new();

    for component in p.components() {
        match component {
            std::path::Component::ParentDir => {
                // Pop but not past the root
                if !components.is_empty() && components.last() != Some(&"..") {
                    let last = components.last().unwrap();
                    // Don't pop root "/"
                    if *last != "/" && *last != "" {
                        components.pop();
                    }
                } else if components.is_empty() {
                    components.push("..");
                }
            }
            std::path::Component::CurDir => {
                // Skip "."
            }
            std::path::Component::RootDir => {
                components.push("/");
            }
            std::path::Component::Normal(c) => {
                components.push(c.to_str().unwrap_or(""));
            }
            std::path::Component::Prefix(_) => {
                // Windows prefix — not supported, keep as-is
            }
        }
    }

    // Build result
    let result = if components.is_empty() {
        if s.starts_with('/') {
            "/".to_string()
        } else {
            ".".to_string()
        }
    } else {
        // If path was absolute (starts with /), components[0] is "/"
        components.join("/")
    };

    // Clean up leading "//" that might occur
    let cleaned = if result.starts_with("//") && !result.starts_with("///") {
        result.replacen("//", "/", 1)
    } else {
        result
    };

    unsafe { str_to_heap(&cleaned) }
}

/// Return the platform-specific path separator (`"/"` on Unix).
#[no_mangle]
pub extern "C" fn __path_separator() -> *mut c_char {
    unsafe { str_to_heap("/") }
}

/// Compute a relative path from `from` to `to`.
///
/// Both inputs are treated as directory paths. The result uses `..` components
/// when ascending from `from` to reach a common ancestor.
#[no_mangle]
pub extern "C" fn __path_relative(from: *const c_char, to: *const c_char) -> *mut c_char {
    let from_str = unsafe { cstr_to_str(from) };
    let to_str = unsafe { cstr_to_str(to) };

    let from_path = Path::new(from_str);
    let to_path = Path::new(to_str);

    // Split into components
    let from_comps: Vec<&str> = from_path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str(),
            std::path::Component::RootDir => Some("/"),
            _ => None,
        })
        .collect();

    let to_comps: Vec<&str> = to_path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str(),
            std::path::Component::RootDir => Some("/"),
            _ => None,
        })
        .collect();

    // Find common prefix length
    let common_len = from_comps
        .iter()
        .zip(to_comps.iter())
        .take_while(|(a, b)| a == b)
        .count();

    // Build result
    let mut result = String::new();
    let up_count = from_comps.len() - common_len;

    // Check for different mount points — can't compute relative path
    let from_has_root = from_str.starts_with('/');
    let to_has_root = to_str.starts_with('/');
    if from_has_root != to_has_root {
        // Different root types — just return `to` as-is
        unsafe {
            return str_to_heap(to_str);
        }
    }
    if from_has_root && to_has_root && common_len == 0 {
        // Both absolute but no common prefix — return `to` as-is
        unsafe {
            return str_to_heap(to_str);
        }
    }

    for _ in 0..up_count {
        if !result.is_empty() {
            result.push('/');
        }
        result.push_str("..");
    }

    for comp in to_comps.iter().skip(common_len) {
        if !result.is_empty() {
            result.push('/');
        }
        result.push_str(comp);
    }

    if result.is_empty() {
        result.push('.');
    }

    unsafe { str_to_heap(&result) }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    // Helper: wrap a Rust string as a C string for test input
    fn c(s: &str) -> *const c_char {
        let c_string = CString::new(s).unwrap();
        c_string.into_raw() as *const c_char
    }

    // Helper: read a C string back to Rust
    unsafe fn to_str(p: *const c_char) -> String {
        if p.is_null() {
            return String::new();
        }
        CStr::from_ptr(p).to_str().unwrap_or("").to_string()
    }

    #[test]
    fn test_join_simple() {
        // Build a mock array handle: [len:3] [cap:3] [ptr1, ptr2, ptr3]
        let seg1 = CString::new("home").unwrap().into_raw();
        let seg2 = CString::new("user").unwrap().into_raw();
        let seg3 = CString::new("docs").unwrap().into_raw();

        unsafe {
            let layout = Layout::from_size_align(
                std::mem::size_of::<i64>() * 2 + std::mem::size_of::<*const c_char>() * 3,
                8,
            )
            .unwrap();
            let arr = alloc(layout) as *mut i64;
            *arr = 3; // length
            *arr.add(1) = 3; // capacity
            let data = arr.add(2) as *mut *const c_char;
            *data = seg1 as *const c_char;
            *data.add(1) = seg2 as *const c_char;
            *data.add(2) = seg3 as *const c_char;

            let result = __path_join(arr as *mut c_char);
            assert_eq!(to_str(result), "home/user/docs");
        }
    }

    #[test]
    fn test_join_empty_segment() {
        let seg1 = CString::new("home").unwrap().into_raw();
        let seg2 = CString::new("").unwrap().into_raw();
        let seg3 = CString::new("docs").unwrap().into_raw();

        unsafe {
            let layout = Layout::from_size_align(
                std::mem::size_of::<i64>() * 2 + std::mem::size_of::<*const c_char>() * 3,
                8,
            )
            .unwrap();
            let arr = alloc(layout) as *mut i64;
            *arr = 3;
            *arr.add(1) = 3;
            let data = arr.add(2) as *mut *const c_char;
            *data = seg1 as *const c_char;
            *data.add(1) = seg2 as *const c_char;
            *data.add(2) = seg3 as *const c_char;

            let result = __path_join(arr as *mut c_char);
            assert_eq!(to_str(result), "home/docs");
        }
    }

    #[test]
    fn test_basename() {
        let result = __path_basename(c("/home/user/file.txt"));
        unsafe {
            assert_eq!(to_str(result), "file.txt");
        }
    }

    #[test]
    fn test_dirname() {
        let result = __path_dirname(c("/home/user/file.txt"));
        unsafe {
            assert_eq!(to_str(result), "/home/user");
        }
    }

    #[test]
    fn test_extname() {
        let result = __path_extname(c("file.tar.gz"));
        unsafe {
            assert_eq!(to_str(result), ".gz");
        }
    }

    #[test]
    fn test_extname_none() {
        let result = __path_extname(c("Makefile"));
        unsafe {
            assert_eq!(to_str(result), "");
        }
    }

    #[test]
    fn test_is_absolute_unix() {
        assert!(__path_is_absolute(c("/usr/bin")));
    }

    #[test]
    fn test_is_absolute_relative() {
        assert!(!__path_is_absolute(c("src/main.rs")));
    }

    #[test]
    fn test_separator_unix() {
        let result = __path_separator();
        unsafe {
            assert_eq!(to_str(result), "/");
        }
    }

    #[test]
    fn test_normalize_dotdot() {
        let result = __path_normalize(c("/a/b/../c/./d"));
        unsafe {
            assert_eq!(to_str(result), "/a/c/d");
        }
    }

    #[test]
    fn test_normalize_relative() {
        let result = __path_normalize(c("./a/b/../c"));
        unsafe {
            assert_eq!(to_str(result), "a/c");
        }
    }

    #[test]
    fn test_relative_sibling() {
        let result = __path_relative(c("/home/user/docs"), c("/home/user/photos/x.jpg"));
        unsafe {
            assert_eq!(to_str(result), "../photos/x.jpg");
        }
    }

    #[test]
    fn test_relative_child() {
        let result = __path_relative(c("/home/user"), c("/home/user/docs/f.txt"));
        unsafe {
            assert_eq!(to_str(result), "docs/f.txt");
        }
    }
}
