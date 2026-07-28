#![allow(clippy::not_unsafe_ptr_arg_deref)]

/**
 * C FFI implementations backing `stdlib/io.ry` and `stdlib/fs.ry`.
 *
 * Provides file and console I/O functions via C ABI. Synchronous operations
 * use Rust stdlib file APIs; async variants spawn OS threads that call the
 * sync version and wrap the result in a Future handle.
 *
 * `io.ry` consumes only `__io_read_line` (console input). All other symbols
 * (`__io_file_*`, `__io_is_*`, `__io_mkdir`, `__io_read_dir`, `__io_remove_dir`,
 * `__io_rename`) are used by `fs.ry` — the single authority for file-system
 * operations.
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
use std::time::UNIX_EPOCH;

// ============================================================
// Helpers
// ============================================================

pub(crate) unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        return "";
    }
    CStr::from_ptr(ptr).to_str().unwrap_or("")
}

pub(crate) unsafe fn str_to_heap(s: &str) -> *mut c_char {
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
// Extended IO (7)
// ============================================================

/// List directory entry names. Returns an Array<string> handle, or null on error.
#[no_mangle]
pub extern "C" fn __io_read_dir(path: *const c_char) -> *mut c_char {
    let p = unsafe { cstr_to_str(path) };
    match fs::read_dir(p) {
        Ok(entries) => {
            let mut names: Vec<*mut c_char> = Vec::new();
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    names.push(unsafe { str_to_heap(name) });
                }
            }
            unsafe { build_string_array(names) }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Return file size in bytes. Returns -1 on error.
#[no_mangle]
pub extern "C" fn __io_file_size(path: *const c_char) -> i64 {
    let p = unsafe { cstr_to_str(path) };
    fs::metadata(p).map(|m| m.len() as i64).unwrap_or(-1)
}

/// Return modification time as Unix epoch milliseconds. Returns -1 on error.
#[no_mangle]
pub extern "C" fn __io_file_mtime(path: *const c_char) -> i64 {
    let p = unsafe { cstr_to_str(path) };
    fs::metadata(p)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(-1)
}

/// Rename (move) a file or directory. Returns true on success.
#[no_mangle]
pub extern "C" fn __io_rename(from: *const c_char, to: *const c_char) -> bool {
    let from_str = unsafe { cstr_to_str(from) };
    let to_str = unsafe { cstr_to_str(to) };
    fs::rename(from_str, to_str).is_ok()
}

/// Remove a directory. If `recursive` is true, removes all contents.
/// Returns true on success.
#[no_mangle]
pub extern "C" fn __io_remove_dir(path: *const c_char, recursive: bool) -> bool {
    let p = unsafe { cstr_to_str(path) };
    if recursive {
        fs::remove_dir_all(p).is_ok()
    } else {
        fs::remove_dir(p).is_ok()
    }
}

/// Append text content to a file. Creates the file if it does not exist.
#[no_mangle]
pub extern "C" fn __io_file_append_text(path: *const c_char, content: *const c_char) {
    let p = unsafe { cstr_to_str(path) };
    let c = unsafe { cstr_to_str(content) };
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(p)
    {
        let _ = std::io::Write::write_all(&mut file, c.as_bytes());
    }
}

/// Read `size` cryptographically-secure random bytes from the OS entropy
/// source (/dev/urandom). Returns a heap-allocated null-terminated buffer,
/// or null on failure (size <= 0, I/O error).
#[no_mangle]
pub extern "C" fn __io_read_random(size: i64) -> *mut c_char {
    use std::alloc::{alloc, Layout};
    use std::io::Read;

    if size <= 0 {
        return std::ptr::null_mut();
    }
    let count = size as usize;
    let total = count + 1;
    let layout = match Layout::from_size_align(total, 1) {
        Ok(l) => l,
        Err(_) => return std::ptr::null_mut(),
    };
    let buf = unsafe { alloc(layout) } as *mut u8;
    if buf.is_null() {
        return std::ptr::null_mut();
    }

    let mut file = match std::fs::File::open("/dev/urandom") {
        Ok(f) => f,
        Err(_) => {
            unsafe {
                std::alloc::dealloc(buf, layout);
            }
            return std::ptr::null_mut();
        }
    };
    let mut read = 0usize;
    let slice = unsafe { std::slice::from_raw_parts_mut(buf, count) };
    while read < count {
        match file.read(&mut slice[read..]) {
            Ok(0) => break,
            Ok(n) => read += n,
            Err(_) => {
                unsafe {
                    std::alloc::dealloc(buf, layout);
                }
                return std::ptr::null_mut();
            }
        }
    }
    unsafe {
        *buf.add(count) = 0;
    }
    buf as *mut c_char
}

// ============================================================
// File streaming I/O — open / read / write / close
// ============================================================

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::Mutex;

static FILE_HANDLES: Mutex<Option<HashMap<i64, File>>> = Mutex::new(None);

fn next_fh() -> i64 {
    use std::sync::atomic::{AtomicI64, Ordering};
    static COUNTER: AtomicI64 = AtomicI64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

fn file_handles() -> std::sync::MutexGuard<'static, Option<HashMap<i64, File>>> {
    let mut guard = FILE_HANDLES.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

#[no_mangle]
pub extern "C" fn __fs_open_read(path: *const c_char) -> i64 {
    let p = unsafe { cstr_to_str(path) };
    match File::open(p) {
        Ok(f) => {
            let h = next_fh();
            file_handles().as_mut().unwrap().insert(h, f);
            h
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn __fs_open_write(path: *const c_char) -> i64 {
    let p = unsafe { cstr_to_str(path) };
    match File::create(p) {
        Ok(f) => {
            let h = next_fh();
            file_handles().as_mut().unwrap().insert(h, f);
            h
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn __fs_open_append(path: *const c_char) -> i64 {
    let p = unsafe { cstr_to_str(path) };
    match std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(p)
    {
        Ok(f) => {
            let h = next_fh();
            file_handles().as_mut().unwrap().insert(h, f);
            h
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn __fs_close(handle: i64) {
    file_handles().as_mut().unwrap().remove(&handle);
}

// ── read / write helpers ─────────────────────────────────────
//
// Array<int> layout (from builtins.rs ruyi_array_alloc):
//   [len: i64][cap: i64][data: i64 * cap]
// Each byte (0-255) is stored as an i64 value in the data section.

pub(crate) unsafe fn array_ptr(ptr: *mut i8) -> (*mut i64, *mut i64, *mut i64) {
    let len_ptr = ptr as *mut i64;
    let cap_ptr = ptr.add(8) as *mut i64;
    let data_ptr = ptr.add(16) as *mut i64;
    (len_ptr, cap_ptr, data_ptr)
}

#[no_mangle]
pub extern "C" fn __fs_read_raw(handle: i64, arr: *mut i8) -> i64 {
    if arr.is_null() {
        return -1;
    }
    let (len_ptr, cap_ptr, data_ptr) = unsafe { array_ptr(arr) };
    let cap = unsafe { *cap_ptr } as usize;
    if cap == 0 {
        return 0;
    }

    let mut guard = file_handles();
    let map = guard.as_mut().unwrap();
    let file = match map.get_mut(&handle) {
        Some(f) => f,
        None => return -2,
    };

    let mut buf = vec![0u8; cap];
    match file.read(&mut buf) {
        Ok(0) => 0,
        Ok(n) => {
            unsafe {
                *len_ptr = n as i64;
                for i in 0..n {
                    *data_ptr.add(i) = buf[i] as i64;
                }
            }
            n as i64
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn __fs_write_raw(handle: i64, arr: *mut i8) -> i64 {
    if arr.is_null() {
        return -1;
    }
    let (len_ptr, _cap_ptr, data_ptr) = unsafe { array_ptr(arr) };
    let len = unsafe { *len_ptr } as usize;
    if len == 0 {
        return 0;
    }

    let mut guard = file_handles();
    let map = guard.as_mut().unwrap();
    let file = match map.get_mut(&handle) {
        Some(f) => f,
        None => return -2,
    };

    let mut buf = Vec::with_capacity(len);
    unsafe {
        for i in 0..len {
            buf.push(*data_ptr.add(i) as u8);
        }
    }
    match file.write_all(&buf) {
        Ok(()) => len as i64,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn __fs_seek(handle: i64, offset: i64, whence: i64) -> i64 {
    let mut guard = file_handles();
    let map = guard.as_mut().unwrap();
    let file = match map.get_mut(&handle) {
        Some(f) => f,
        None => return -2,
    };
    let pos = match whence {
        0 => SeekFrom::Start(offset as u64),
        1 => SeekFrom::Current(offset),
        2 => SeekFrom::End(offset),
        _ => return -1,
    };
    match file.seek(pos) {
        Ok(p) => p as i64,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn __fs_tell(handle: i64) -> i64 {
    let mut guard = file_handles();
    let map = guard.as_mut().unwrap();
    let file = match map.get_mut(&handle) {
        Some(f) => f,
        None => return -2,
    };
    match file.stream_position() {
        Ok(p) => p as i64,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn __fs_flush(handle: i64) -> i64 {
    let mut guard = file_handles();
    let map = guard.as_mut().unwrap();
    let file = match map.get_mut(&handle) {
        Some(f) => f,
        None => return -2,
    };
    match file.flush() {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

// ============================================================
// Stdio streaming — stdin / stdout raw I/O
// ============================================================

use std::io::{stdin, stdout};

#[no_mangle]
pub extern "C" fn __io_read_raw(arr: *mut i8) -> i64 {
    if arr.is_null() {
        return -1;
    }
    let (len_ptr, cap_ptr, data_ptr) = unsafe { array_ptr(arr) };
    let cap = unsafe { *cap_ptr } as usize;
    if cap == 0 {
        return 0;
    }

    let mut buf = vec![0u8; cap];
    let mut lock = stdin().lock();
    match lock.read(&mut buf) {
        Ok(0) => 0,
        Ok(n) => {
            unsafe {
                *len_ptr = n as i64;
                for i in 0..n {
                    *data_ptr.add(i) = buf[i] as i64;
                }
            }
            n as i64
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn __io_write_raw(arr: *mut i8) -> i64 {
    if arr.is_null() {
        return -1;
    }
    let (len_ptr, _cap_ptr, data_ptr) = unsafe { array_ptr(arr) };
    let len = unsafe { *len_ptr } as usize;
    if len == 0 {
        return 0;
    }

    let mut buf = Vec::with_capacity(len);
    unsafe {
        for i in 0..len {
            buf.push(*data_ptr.add(i) as u8);
        }
    }
    let mut lock = stdout().lock();
    match lock.write_all(&buf) {
        Ok(()) => len as i64,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn __io_flush() -> i64 {
    match stdout().flush() {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn __io_write_stderr_raw(arr: *mut i8) -> i64 {
    if arr.is_null() {
        return -1;
    }
    let (len_ptr, _cap_ptr, data_ptr) = unsafe { array_ptr(arr) };
    let len = unsafe { *len_ptr } as usize;
    if len == 0 {
        return 0;
    }

    let mut buf = Vec::with_capacity(len);
    unsafe {
        for i in 0..len {
            buf.push(*data_ptr.add(i) as u8);
        }
    }
    let mut lock = std::io::stderr().lock();
    match lock.write_all(&buf) {
        Ok(()) => len as i64,
        Err(_) => -1,
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

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
        assert!(!result.is_null());
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

    #[test]
    fn test_read_dir() {
        let base = temp_path("read_dir");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        fs::write(format!("{}/a.txt", base), "x").unwrap();
        fs::write(format!("{}/b.txt", base), "y").unwrap();
        fs::create_dir(format!("{}/sub", base)).unwrap();

        let result = __io_read_dir(c(&base));
        unsafe {
            assert!(!result.is_null());
            // Array layout: [len: i64][cap: i64][ptr0, ...]
            let arr = result as *const i64;
            let len = *arr;
            assert!(len >= 2);
        }
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_read_dir_nonexistent() {
        let result = __io_read_dir(c("/nonexistent/ruyi_test_dir_xyz"));
        assert!(result.is_null());
    }

    #[test]
    fn test_file_size() {
        let path = temp_path("file_size.txt");
        fs::write(&path, "hello").unwrap();
        let size = __io_file_size(c(&path));
        assert_eq!(size, 5);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_file_size_nonexistent() {
        let size = __io_file_size(c("/nonexistent/ruyi_test_size_xyz"));
        assert_eq!(size, -1);
    }

    #[test]
    fn test_file_mtime() {
        let path = temp_path("file_mtime.txt");
        fs::write(&path, "hello").unwrap();
        let mtime = __io_file_mtime(c(&path));
        assert!(mtime > 0);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_file_mtime_nonexistent() {
        let mtime = __io_file_mtime(c("/nonexistent/ruyi_test_mtime_xyz"));
        assert_eq!(mtime, -1);
    }

    #[test]
    fn test_rename_success() {
        let from = temp_path("rename_from.txt");
        let to = temp_path("rename_to.txt");
        fs::write(&from, "hello").unwrap();
        let _ = fs::remove_file(&to);

        assert!(__io_rename(c(&from), c(&to)));
        assert!(!Path::new(&from).exists());
        assert!(Path::new(&to).exists());
        assert_eq!(fs::read_to_string(&to).unwrap(), "hello");
        let _ = fs::remove_file(&to);
    }

    #[test]
    fn test_rename_nonexistent() {
        assert!(!__io_rename(
            c("/nonexistent/ruyi_rename_src_xyz"),
            c("/nonexistent/ruyi_rename_dst_xyz"),
        ));
    }

    #[test]
    fn test_remove_dir_empty() {
        let path = temp_path("rmdir_empty");
        let _ = fs::remove_dir_all(&path);
        fs::create_dir(&path).unwrap();
        assert!(__io_remove_dir(c(&path), false));
        assert!(!Path::new(&path).exists());
    }

    #[test]
    fn test_remove_dir_recursive() {
        let base = temp_path("rmdir_rec");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(format!("{}/a/b", base)).unwrap();
        fs::write(format!("{}/a/b/x.txt", base), "x").unwrap();

        assert!(__io_remove_dir(c(&base), true));
        assert!(!Path::new(&base).exists());
    }

    #[test]
    fn test_file_append_text_new() {
        let path = temp_path("append_new.txt");
        let _ = fs::remove_file(&path);
        __io_file_append_text(c(&path), c("hello"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_file_append_text_existing() {
        let path = temp_path("append_existing.txt");
        fs::write(&path, "hello").unwrap();
        __io_file_append_text(c(&path), c(" world"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello world");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_read_random_positive() {
        let result = __io_read_random(32);
        assert!(
            !result.is_null(),
            "should return non-null buffer for size=32"
        );
        unsafe {
            let slice = std::slice::from_raw_parts(result as *const u8, 32);
            // Statistically impossible for CSPRNG to produce all zeros
            let all_zero = slice.iter().all(|&b| b == 0);
            assert!(!all_zero, "CSPRNG should not produce all zeros");
        }
    }

    #[test]
    fn test_read_random_zero_and_negative() {
        assert!(__io_read_random(0).is_null(), "null for size=0");
        assert!(__io_read_random(-5).is_null(), "null for size=-5");
    }
}
