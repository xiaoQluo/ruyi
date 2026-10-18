#![allow(clippy::not_unsafe_ptr_arg_deref)]

/**
 * C FFI implementations backing `stdlib/process.ry`.
 *
 * Provides process management, environment variable access, and system
 * information functions via C ABI. Uses `std::process::Command` for
 * process execution and spawning.
 *
 * @author Ruyi Team
 * @date 2026-07-17
 */
use std::alloc::{alloc, Layout};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

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

/// Opaque Process handle wrapping `std::process::Child`.
struct ProcessHandle {
    child: Mutex<Option<Child>>,
    exit_code: Mutex<Option<i32>>,
    #[allow(dead_code)]
    stdout_buf: Mutex<Vec<u8>>,
    #[allow(dead_code)]
    stderr_buf: Mutex<Vec<u8>>,
}

/// Build a `Command` for `cmd_str`, routed through the system shell when
/// `shell` is set and split on whitespace otherwise.
fn build_command(cmd_str: &str, shell: bool) -> Command {
    if shell {
        if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", cmd_str]);
            c
        } else {
            let mut c = Command::new("sh");
            c.arg("-c").arg(cmd_str);
            c
        }
    } else {
        let mut parts = cmd_str.split_whitespace();
        let mut c = Command::new(parts.next().unwrap_or(""));
        for arg in parts {
            c.arg(arg);
        }
        c
    }
}

// ============================================================
// Synchronous Execution Capture
// ============================================================

/// Output of one synchronous command run.
///
/// stdout, stderr and the exit code each have to reach Ruyi as a value of
/// its own. They cannot share a single packed C string: Ruyi strings are
/// NUL-terminated, so a `\0`-delimited triple would read back as its first
/// field alone, and returning that pointer as if it were a `ProcessResult`
/// instance made every field load land inside the packed bytes.
struct ExecCapture {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit_code: i64,
}

impl ExecCapture {
    fn from_output(out: std::process::Output) -> Self {
        ExecCapture {
            stdout: out.stdout,
            stderr: out.stderr,
            exit_code: out.status.code().unwrap_or(-1) as i64,
        }
    }

    fn stream(&self, stderr: bool) -> String {
        let bytes = if stderr { &self.stderr } else { &self.stdout };
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// Run `command` to completion and capture its output.
///
/// `cwd` may be null or empty to inherit the current directory. Returns an
/// opaque capture handle, or 0 when the command could not be spawned.
#[no_mangle]
pub extern "C" fn __process_exec_capture(
    command: *const c_char,
    cwd: *const c_char,
    shell: bool,
) -> i64 {
    let cmd_str = unsafe { cstr_to_str(command) };
    let cwd_str = unsafe { cstr_to_str(cwd) };
    let mut cmd = build_command(cmd_str, shell);
    if !cwd_str.is_empty() {
        cmd.current_dir(cwd_str);
    }
    match cmd.output() {
        Ok(out) => Box::into_raw(Box::new(ExecCapture::from_output(out))) as i64,
        Err(_) => 0,
    }
}

/// Read a captured stream, returning an empty string for a null handle.
unsafe fn capture_stream(handle: i64, stderr: bool) -> *mut c_char {
    if handle == 0 {
        return str_to_heap("");
    }
    let capture = &*(handle as *const ExecCapture);
    str_to_heap(&capture.stream(stderr))
}

#[no_mangle]
pub extern "C" fn __process_capture_stdout(handle: i64) -> *mut c_char {
    unsafe { capture_stream(handle, false) }
}

#[no_mangle]
pub extern "C" fn __process_capture_stderr(handle: i64) -> *mut c_char {
    unsafe { capture_stream(handle, true) }
}

/// Exit code of a captured run, or -1 for a null handle.
#[no_mangle]
pub extern "C" fn __process_capture_code(handle: i64) -> i64 {
    if handle == 0 {
        return -1;
    }
    unsafe { (*(handle as *const ExecCapture)).exit_code }
}

/// Release a capture handle. Null handles are ignored.
#[no_mangle]
pub extern "C" fn __process_capture_free(handle: i64) {
    if handle == 0 {
        return;
    }
    unsafe {
        drop(Box::from_raw(handle as *mut ExecCapture));
    }
}

/// Spawn `command` and return an opaque handle, or 0 on failure.
///
/// The handle is an integer rather than a pointer because Ruyi types every
/// `__process_*` handle as `int`, matching `FileReader._h`.
#[no_mangle]
pub extern "C" fn __process_create(
    command: *const c_char,
    cwd: *const c_char,
    _env: *mut c_char,
    shell: bool,
) -> i64 {
    let cmd_str = unsafe { cstr_to_str(command) }.to_string();
    let cwd_str = unsafe { cstr_to_str(cwd) }.to_string();

    let mut cmd = build_command(&cmd_str, shell);
    if !cwd_str.is_empty() {
        cmd.current_dir(&cwd_str);
    }

    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    match cmd.spawn() {
        Ok(child) => {
            let handle = Box::new(ProcessHandle {
                child: Mutex::new(Some(child)),
                exit_code: Mutex::new(None),
                stdout_buf: Mutex::new(Vec::new()),
                stderr_buf: Mutex::new(Vec::new()),
            });
            Box::into_raw(handle) as i64
        }
        Err(_) => 0,
    }
}

/// OS pid of a spawned child, or -1 for a null handle.
#[no_mangle]
pub extern "C" fn __process_handle_pid(proc: i64) -> i64 {
    if proc == 0 {
        return -1;
    }
    unsafe {
        let handle = &*(proc as *const ProcessHandle);
        match handle.child.lock() {
            Ok(child_opt) => match child_opt.as_ref() {
                Some(child) => child.id() as i64,
                None => -1,
            },
            Err(_) => -1,
        }
    }
}

/// Release a process handle. Null handles are ignored.
#[no_mangle]
pub extern "C" fn __process_handle_free(proc: i64) {
    if proc == 0 {
        return;
    }
    unsafe {
        drop(Box::from_raw(proc as *mut ProcessHandle));
    }
}

// ============================================================
// Process Lifecycle
// ============================================================

#[no_mangle]
pub extern "C" fn __process_wait(proc: i64) -> i64 {
    if proc == 0 {
        return -1;
    }
    unsafe {
        let handle = &*(proc as *const ProcessHandle);
        if let Ok(ec) = handle.exit_code.lock() {
            if let Some(code) = *ec {
                return code as i64;
            }
        }
        if let Ok(mut child_opt) = handle.child.lock() {
            if let Some(ref mut child) = *child_opt {
                match child.wait() {
                    Ok(status) => {
                        let code = status.code().unwrap_or(-1);
                        if let Ok(mut ec) = handle.exit_code.lock() {
                            *ec = Some(code);
                        }
                        code as i64
                    }
                    Err(_) => -1,
                }
            } else {
                -1
            }
        } else {
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn __process_wait_async(proc: i64) -> i64 {
    __process_wait(proc)
}

#[no_mangle]
pub extern "C" fn __process_kill(proc: i64, _signal: i64) {
    if proc == 0 {
        return;
    }
    unsafe {
        let handle = &*(proc as *const ProcessHandle);
        if let Ok(mut child_opt) = handle.child.lock() {
            if let Some(ref mut child) = *child_opt {
                let _ = child.kill();
            }
        }
    }
}

// ============================================================
// Process I/O
// ============================================================

#[no_mangle]
pub extern "C" fn __process_write_input(proc: i64, input: *const c_char) {
    if proc == 0 {
        return;
    }
    let data = unsafe { cstr_to_str(input) };
    unsafe {
        let handle = &mut *(proc as *mut ProcessHandle);
        if let Ok(mut child_opt) = handle.child.lock() {
            if let Some(ref mut child) = *child_opt {
                if let Some(ref mut stdin) = child.stdin {
                    use std::io::Write;
                    let _ = stdin.write_all(data.as_bytes());
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn __process_close_input(proc: i64) {
    if proc == 0 {
        return;
    }
    unsafe {
        let handle = &mut *(proc as *mut ProcessHandle);
        if let Ok(mut child_opt) = handle.child.lock() {
            if let Some(ref mut child) = *child_opt {
                child.stdin.take();
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn __process_read_output(proc: i64) -> *mut c_char {
    if proc == 0 {
        return std::ptr::null_mut();
    }
    unsafe {
        let handle = &mut *(proc as *mut ProcessHandle);
        if let Ok(mut child_opt) = handle.child.lock() {
            if let Some(ref mut child) = *child_opt {
                if let Some(ref mut stdout) = child.stdout {
                    use std::io::Read;
                    let mut buf = [0u8; 4096];
                    match stdout.read(&mut buf) {
                        Ok(0) => return std::ptr::null_mut(),
                        Ok(n) => {
                            let s = String::from_utf8_lossy(&buf[..n]).to_string();
                            return str_to_heap(&s);
                        }
                        Err(_) => return std::ptr::null_mut(),
                    }
                }
            }
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn __process_read_error(proc: i64) -> *mut c_char {
    if proc == 0 {
        return std::ptr::null_mut();
    }
    unsafe {
        let handle = &mut *(proc as *mut ProcessHandle);
        if let Ok(mut child_opt) = handle.child.lock() {
            if let Some(ref mut child) = *child_opt {
                if let Some(ref mut stderr) = child.stderr {
                    use std::io::Read;
                    let mut buf = [0u8; 4096];
                    match stderr.read(&mut buf) {
                        Ok(0) => return std::ptr::null_mut(),
                        Ok(n) => {
                            let s = String::from_utf8_lossy(&buf[..n]).to_string();
                            return str_to_heap(&s);
                        }
                        Err(_) => return std::ptr::null_mut(),
                    }
                }
            }
        }
    }
    std::ptr::null_mut()
}

// ============================================================
// Environment Variables
// ============================================================

#[no_mangle]
pub extern "C" fn __process_get_env(name: *const c_char) -> *mut c_char {
    let n = unsafe { cstr_to_str(name) };
    match std::env::var(n) {
        Ok(val) => unsafe { str_to_heap(&val) },
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn __process_set_env(name: *const c_char, value: *const c_char) {
    let n = unsafe { cstr_to_str(name) };
    let v = unsafe { cstr_to_str(value) };
    std::env::set_var(n, v);
}

#[no_mangle]
pub extern "C" fn __process_get_all_env() -> *mut c_char {
    // Return a simple representation: "KEY1=VAL1\nKEY2=VAL2\n..."
    let mut result = String::new();
    for (key, value) in std::env::vars() {
        result.push_str(&format!("{}={}\n", key, value));
    }
    unsafe { str_to_heap(&result) }
}

// ============================================================
// System Information
// ============================================================

/**
 * Terminate the current process with the given exit status.
 *
 * @param code Status reported to the parent process; 0 means success.
 */
#[no_mangle]
pub extern "C" fn __process_exit(code: i64) {
    std::process::exit(code as i32);
}

#[no_mangle]
pub extern "C" fn __process_get_pid() -> i64 {
    std::process::id() as i64
}

#[no_mangle]
pub extern "C" fn __process_get_ppid() -> i64 {
    // Use `ps` command to get parent PID (Unix only)
    let output = Command::new("ps")
        .args(["-o", "ppid=", "-p", &std::process::id().to_string()])
        .output();
    match output {
        Ok(out) => {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            s.parse().unwrap_or(-1)
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn __process_get_platform() -> *mut c_char {
    let platform = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    };
    unsafe { str_to_heap(platform) }
}

#[no_mangle]
pub extern "C" fn __process_get_cpu_count() -> i64 {
    std::thread::available_parallelism()
        .map(|n| n.get() as i64)
        .unwrap_or(1)
}

#[no_mangle]
pub extern "C" fn __process_get_total_memory() -> i64 {
    get_memory_info().0
}

#[no_mangle]
pub extern "C" fn __process_get_free_memory() -> i64 {
    get_memory_info().1
}

fn get_memory_info() -> (i64, i64) {
    // Use system commands to query memory (no libc dependency)
    let total = Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<i64>()
                .ok()
        })
        .unwrap_or(0);

    // Estimate free memory via vm_stat on macOS, /proc/meminfo on Linux
    #[cfg(target_os = "macos")]
    let free = {
        // 动态获取页面大小，Intel Mac 为 4096，Apple Silicon 为 16384
        let page_size: i64 = Command::new("sysctl")
            .args(["-n", "hw.pagesize"])
            .output()
            .ok()
            .and_then(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .parse::<i64>()
                    .ok()
            })
            .unwrap_or(4096);

        Command::new("vm_stat")
            .output()
            .ok()
            .map(|o| {
                let out = String::from_utf8_lossy(&o.stdout);
                for line in out.lines() {
                    if line.starts_with("Pages free:") {
                        let pages: i64 = line
                            .split_whitespace()
                            .last()
                            .unwrap_or("0")
                            .trim_end_matches('.')
                            .parse()
                            .unwrap_or(0);
                        return pages * page_size;
                    }
                }
                total / 4
            })
            .unwrap_or(total / 4)
    };

    #[cfg(target_os = "linux")]
    let free = {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .map(|content| {
                for line in content.lines() {
                    if line.starts_with("MemAvailable:") {
                        let kb: i64 = line
                            .split_whitespace()
                            .nth(1)
                            .unwrap_or("0")
                            .parse()
                            .unwrap_or(0);
                        return kb * 1024;
                    }
                }
                0
            })
            .unwrap_or(0)
    };

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let free = 0;

    (total, free)
}

// ============================================================
// Signal Handling
// ============================================================

#[no_mangle]
pub extern "C" fn __process_signal_available(signal: i64) -> bool {
    #[cfg(not(target_os = "windows"))]
    {
        matches!(signal, 1 | 2 | 3 | 9 | 10 | 12 | 15)
    }
    #[cfg(target_os = "windows")]
    {
        false
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

    #[test]
    fn test_exec_capture_echo() {
        let h = __process_exec_capture(c("echo hello"), std::ptr::null(), true);
        assert_ne!(h, 0, "echo must produce a capture handle");
        unsafe {
            assert_eq!(to_str(__process_capture_stdout(h)).trim(), "hello");
            assert_eq!(to_str(__process_capture_stderr(h)), "");
        }
        assert_eq!(__process_capture_code(h), 0);
        __process_capture_free(h);
    }

    #[test]
    fn test_exec_capture_failure() {
        let h = __process_exec_capture(c("exit 7"), std::ptr::null(), true);
        assert_eq!(__process_capture_code(h), 7);
        __process_capture_free(h);
    }

    #[test]
    fn test_exec_capture_spawn_error_has_no_handle() {
        let h = __process_exec_capture(c("echo x"), c("/nonexistent_dir_xyz_123"), true);
        assert_eq!(h, 0, "a bad cwd must not produce a handle");
    }

    // stdout and stderr must stay separate; a single packed string cannot
    // carry both because Ruyi strings are NUL-terminated.
    #[test]
    fn test_exec_capture_keeps_streams_separate() {
        let h = __process_exec_capture(c("echo out; echo err 1>&2"), std::ptr::null(), true);
        unsafe {
            assert_eq!(to_str(__process_capture_stdout(h)).trim(), "out");
            assert_eq!(to_str(__process_capture_stderr(h)).trim(), "err");
        }
        __process_capture_free(h);
    }

    #[test]
    fn test_capture_accessors_reject_null_handle() {
        unsafe {
            assert_eq!(to_str(__process_capture_stdout(0)), "");
            assert_eq!(to_str(__process_capture_stderr(0)), "");
        }
        assert_eq!(__process_capture_code(0), -1);
        __process_capture_free(0);
    }

    #[test]
    fn test_free_on_zero_handle_is_safe() {
        __process_capture_free(0);
        __process_handle_free(0);
    }

    #[test]
    fn test_create_and_wait() {
        let proc = __process_create(c("echo test"), std::ptr::null(), std::ptr::null_mut(), true);
        assert_ne!(proc, 0);
        assert!(__process_handle_pid(proc) > 0);
        let code = __process_wait(proc);
        assert_eq!(code, 0);
        __process_handle_free(proc);
    }

    #[test]
    fn test_create_and_kill() {
        let proc = __process_create(c("sleep 10"), std::ptr::null(), std::ptr::null_mut(), true);
        assert_ne!(proc, 0);
        __process_kill(proc, 9);
        let code = __process_wait(proc);
        // Killed process should have non-zero exit
        assert!(code != 0 || code == -1);
        __process_handle_free(proc);
    }

    #[test]
    fn test_get_pid() {
        let pid = __process_get_pid();
        assert!(pid > 0);
    }

    #[test]
    fn test_get_platform() {
        let platform = __process_get_platform();
        unsafe {
            let s = to_str(platform);
            assert!(s == "macos" || s == "linux" || s == "windows");
        }
    }

    #[test]
    fn test_get_cpu_count() {
        let n = __process_get_cpu_count();
        assert!(n >= 1);
    }

    #[test]
    fn test_get_memory() {
        let total = __process_get_total_memory();
        let free = __process_get_free_memory();
        assert!(total >= 0);
        assert!(free >= 0);
        assert!(total >= free, "total memory should be >= free memory");
    }

    #[test]
    fn test_get_env_home() {
        let result = __process_get_env(c("HOME"));
        unsafe {
            if !result.is_null() {
                assert!(!to_str(result).is_empty());
            }
        }
    }

    #[test]
    fn test_get_env_missing() {
        let result = __process_get_env(c("RUYI_NONEXISTENT_VAR_XYZ_12345"));
        assert!(result.is_null());
    }

    #[test]
    fn test_set_and_get_env() {
        __process_set_env(c("RUYI_TEST_X"), c("42"));
        let result = __process_get_env(c("RUYI_TEST_X"));
        unsafe {
            assert!(!result.is_null());
            assert_eq!(to_str(result), "42");
        }
    }

    #[test]
    fn test_get_all_env() {
        let result = __process_get_all_env();
        unsafe {
            assert!(!result.is_null());
            let s = to_str(result);
            assert!(!s.is_empty());
        }
    }

    #[test]
    fn test_signal_available() {
        assert!(__process_signal_available(9));
        assert!(__process_signal_available(15));
        #[cfg(not(target_os = "windows"))]
        assert!(!__process_signal_available(999));
    }

    #[test]
    fn test_write_input_and_read_output() {
        let proc = __process_create(c("cat"), std::ptr::null(), std::ptr::null_mut(), true);
        assert_ne!(proc, 0);
        __process_write_input(proc, c("hello_from_test"));
        __process_close_input(proc);

        // Read output after closing input
        std::thread::sleep(std::time::Duration::from_millis(200));
        let out = __process_read_output(proc);
        assert!(!out.is_null(), "cat should have echoed the input");
        unsafe {
            assert_eq!(to_str(out), "hello_from_test");
        }
        // cat should echo back the input
        let code = __process_wait(proc);
        assert_eq!(code, 0);
        __process_handle_free(proc);
    }
}
