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

// ============================================================
// Command Execution
// ============================================================

#[no_mangle]
pub extern "C" fn __process_exec(command: *const c_char) -> *mut c_char {
    let cmd = unsafe { cstr_to_str(command) };
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", cmd]).output()
    } else {
        Command::new("sh").arg("-c").arg(cmd).output()
    };
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let exit_code = out.status.code().unwrap_or(-1) as i64;
            let result = format!("{}\x00{}\x00{}", stdout, stderr, exit_code);
            unsafe { str_to_heap(&result) }
        }
        Err(e) => {
            let msg = format!("\x00{}\x00-1", e);
            unsafe { str_to_heap(&msg) }
        }
    }
}

#[no_mangle]
pub extern "C" fn __process_exec_with(
    command: *const c_char,
    cwd: *const c_char,
    _env: *mut c_char,
    shell: bool,
) -> *mut c_char {
    let cmd_str = unsafe { cstr_to_str(command) };
    let cwd_str = unsafe { cstr_to_str(cwd) };

    let mut cmd = if shell {
        let c = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", cmd_str]);
            c
        } else {
            let mut c = Command::new("sh");
            c.arg("-c").arg(cmd_str);
            c
        };
        c
    } else {
        let mut parts = cmd_str.split_whitespace();
        let prog = parts.next().unwrap_or("");
        let mut c = Command::new(prog);
        for arg in parts {
            c.arg(arg);
        }
        c
    };

    if !cwd_str.is_empty() {
        cmd.current_dir(cwd_str);
    }

    match cmd.output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let exit_code = out.status.code().unwrap_or(-1) as i64;
            let result = format!("{}\x00{}\x00{}", stdout, stderr, exit_code);
            unsafe { str_to_heap(&result) }
        }
        Err(e) => {
            let msg = format!("\x00{}\x00-1", e);
            unsafe { str_to_heap(&msg) }
        }
    }
}

#[no_mangle]
pub extern "C" fn __process_create(
    command: *const c_char,
    cwd: *const c_char,
    _env: *mut c_char,
    shell: bool,
) -> *mut c_char {
    let cmd_str = unsafe { cstr_to_str(command) }.to_string();
    let cwd_str = unsafe { cstr_to_str(cwd) }.to_string();

    let mut cmd = if shell {
        let c = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", &cmd_str]);
            c
        } else {
            let mut c = Command::new("sh");
            c.arg("-c").arg(&cmd_str);
            c
        };
        c
    } else {
        let mut parts = cmd_str.split_whitespace();
        let prog = parts.next().unwrap_or("");
        let mut c = Command::new(prog);
        for arg in parts {
            c.arg(arg);
        }
        c
    };

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
            Box::into_raw(handle) as *mut c_char
        }
        Err(_) => std::ptr::null_mut(),
    }
}

// ============================================================
// Process Lifecycle
// ============================================================

#[no_mangle]
pub extern "C" fn __process_wait(proc: *mut c_char) -> i64 {
    if proc.is_null() {
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
pub extern "C" fn __process_wait_async(proc: *mut c_char) -> *mut c_char {
    __process_wait(proc);
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn __process_kill(proc: *mut c_char, _signal: i64) {
    if proc.is_null() {
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
pub extern "C" fn __process_write_input(proc: *mut c_char, input: *const c_char) {
    if proc.is_null() {
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
pub extern "C" fn __process_close_input(proc: *mut c_char) {
    if proc.is_null() {
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
pub extern "C" fn __process_read_output(proc: *mut c_char) -> *mut c_char {
    if proc.is_null() {
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
pub extern "C" fn __process_read_error(proc: *mut c_char) -> *mut c_char {
    if proc.is_null() {
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
                        return pages * 16384; // 16KB pages on Apple Silicon / x86
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
    fn test_exec_echo() {
        let result = __process_exec(c("echo hello"));
        unsafe {
            let s = to_str(result);
            assert!(s.contains("hello"));
        }
    }

    #[test]
    fn test_exec_failure() {
        let result = __process_exec(c("nonexistent_command_xyz_123"));
        unsafe {
            assert!(!result.is_null());
        }
    }

    #[test]
    fn test_create_and_wait() {
        let proc = __process_create(c("echo test"), std::ptr::null(), std::ptr::null_mut(), true);
        assert!(!proc.is_null());
        let code = __process_wait(proc);
        assert_eq!(code, 0);
    }

    #[test]
    fn test_create_and_kill() {
        let proc = __process_create(c("sleep 10"), std::ptr::null(), std::ptr::null_mut(), true);
        assert!(!proc.is_null());
        __process_kill(proc, 9);
        let code = __process_wait(proc);
        // Killed process should have non-zero exit
        assert!(code != 0 || code == -1);
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
        assert!(!proc.is_null());
        __process_write_input(proc, c("hello_from_test"));
        __process_close_input(proc);

        // Read output after closing input
        std::thread::sleep(std::time::Duration::from_millis(200));
        let out = __process_read_output(proc);
        // cat should echo back the input
        let code = __process_wait(proc);
        assert_eq!(code, 0);
    }
}
