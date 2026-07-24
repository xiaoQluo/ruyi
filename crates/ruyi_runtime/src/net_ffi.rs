/**
 * C FFI implementations backing `stdlib/net.ry`.
 *
 * Provides TCP socket operations via C ABI. Socket and server handles are
 * represented as `i64` opaque IDs tracked in global registries. All I/O
 * is synchronous (blocking) — async wrappers live in the .ry layer.
 *
 * @author Ruyi Team
 * @date 2026-07-18
 */
use std::alloc::{alloc, Layout};
use std::collections::HashMap;
use std::ffi::CStr;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::os::raw::c_char;
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

// ============================================================
// Global Socket / Server Registries
// ============================================================

static SOCKETS: Mutex<Option<HashMap<i64, TcpStream>>> = Mutex::new(None);
static LISTENERS: Mutex<Option<HashMap<i64, TcpListener>>> = Mutex::new(None);
static UDP_SOCKETS: Mutex<Option<HashMap<i64, UdpState>>> = Mutex::new(None);
static NEXT_HANDLE: Mutex<i64> = Mutex::new(0);

struct UdpState {
    socket: UdpSocket,
    sender_host: String,
    sender_port: i64,
}

fn next_handle() -> i64 {
    let mut h = NEXT_HANDLE.lock().unwrap();
    *h += 1;
    *h
}

// ============================================================
// TCP Client — Socket Operations
// ============================================================

/// Connect to `host:port` and return a positive handle, or a negative
/// error code on failure.
#[no_mangle]
pub extern "C" fn __net_tcp_connect(host: *const c_char, port: i64) -> i64 {
    let h = unsafe { cstr_to_str(host) };
    let addr = format!("{}:{}", h, port);
    match TcpStream::connect(&addr) {
        Ok(stream) => {
            let handle = next_handle();
            let mut map = SOCKETS.lock().unwrap();
            if map.is_none() {
                *map = Some(HashMap::new());
            }
            map.as_mut().unwrap().insert(handle, stream);
            handle
        }
        Err(_) => -1,
    }
}

/// Read up to `max_bytes` from socket handle. Returns heap-allocated data
/// string, or null / empty on EOF / error.
#[no_mangle]
pub extern "C" fn __net_tcp_read(handle: i64, max_bytes: i64) -> *mut c_char {
    let mut map = SOCKETS.lock().unwrap();
    if map.is_none() {
        return unsafe { str_to_heap("") };
    }
    let streams = map.as_mut().unwrap();
    if let Some(stream) = streams.get_mut(&handle) {
        let mut buf = vec![0u8; max_bytes as usize];
        match stream.read(&mut buf) {
            Ok(0) => unsafe { str_to_heap("") },          // EOF
            Ok(n) => unsafe { str_to_heap(&String::from_utf8_lossy(&buf[..n])) },
            Err(_) => unsafe { str_to_heap("") },
        }
    } else {
        unsafe { str_to_heap("") }
    }
}

/// Write data to socket handle. Returns bytes written, or -1 on error.
#[no_mangle]
pub extern "C" fn __net_tcp_write(handle: i64, data: *const c_char) -> i64 {
    let d = unsafe { cstr_to_str(data) };
    let mut map = SOCKETS.lock().unwrap();
    if map.is_none() {
        return -1;
    }
    let streams = map.as_mut().unwrap();
    if let Some(stream) = streams.get_mut(&handle) {
        match stream.write(d.as_bytes()) {
            Ok(n) => n as i64,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn __net_tcp_write_raw(handle: i64, data: *const u8, len: i64) -> i64 {
    if data.is_null() || len <= 0 { return 0; }
    let buf = unsafe { std::slice::from_raw_parts(data, len as usize) };
    let mut map = SOCKETS.lock().unwrap();
    if map.is_none() { return -1; }
    let streams = map.as_mut().unwrap();
    if let Some(stream) = streams.get_mut(&handle) {
        match stream.write(buf) {
            Ok(n) => n as i64,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

/// Read raw bytes from socket handle into a Ruyi Array<int>.
/// The array's capacity determines the max read size.
/// Returns bytes actually read (0 = EOF), or -1/-2 on error.
#[no_mangle]
pub extern "C" fn __net_tcp_read_raw(handle: i64, arr: *mut i8) -> i64 {
    if arr.is_null() { return -1; }
    let (len_ptr, cap_ptr, data_ptr) = unsafe { crate::io_ffi::array_ptr(arr) };
    let cap = unsafe { *cap_ptr } as usize;
    if cap == 0 { return 0; }

    let mut map = SOCKETS.lock().unwrap();
    if map.is_none() { return -1; }
    let streams = map.as_mut().unwrap();
    match streams.get_mut(&handle) {
        Some(stream) => {
            let mut buf = vec![0u8; cap];
            match stream.read(&mut buf) {
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
        None => -2,
    }
}

/// Close socket handle and remove from registry.
#[no_mangle]
pub extern "C" fn __net_tcp_close(handle: i64) {
    let mut map = SOCKETS.lock().unwrap();
    if let Some(ref mut streams) = *map {
        streams.remove(&handle);
    }
}

// ── Binary I/O helpers for internal use (TLS FFI, etc.) ──

pub(crate) fn tcp_read_raw(handle: i64, buf: &mut [u8]) -> i64 {
    let mut map = SOCKETS.lock().unwrap();
    if map.is_none() {
        return -1;
    }
    let streams = map.as_mut().unwrap();
    if let Some(stream) = streams.get_mut(&handle) {
        match stream.read(buf) {
            Ok(n) => n as i64,
            Err(_) => -1,
        }
    } else {
        -2
    }
}

pub(crate) fn tcp_write_raw(handle: i64, buf: &[u8]) -> i64 {
    let mut map = SOCKETS.lock().unwrap();
    if map.is_none() {
        return -1;
    }
    let streams = map.as_mut().unwrap();
    if let Some(stream) = streams.get_mut(&handle) {
        match stream.write(buf) {
            Ok(n) => n as i64,
            Err(_) => -1,
        }
    } else {
        -2
    }
}

// ── end helpers ──

/// Set socket read timeout in milliseconds. 0 means no timeout (blocking).
/// Returns 0 on success, -1 if handle not found.
#[no_mangle]
pub extern "C" fn __net_tcp_set_timeout(handle: i64, timeout_ms: i64) -> i64 {
    let mut map = SOCKETS.lock().unwrap();
    if map.is_none() {
        return -1;
    }
    let streams = map.as_mut().unwrap();
    if let Some(stream) = streams.get_mut(&handle) {
        let dur = std::time::Duration::from_millis(timeout_ms as u64);
        match stream.set_read_timeout(Some(dur)).and(stream.set_write_timeout(Some(dur))) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

// ============================================================
// TCP Server Operations
// ============================================================

/// Bind and listen on `host:port`. Returns a positive handle, or -1 on error.
#[no_mangle]
pub extern "C" fn __net_tcp_listen(host: *const c_char, port: i64) -> i64 {
    let h = unsafe { cstr_to_str(host) };
    let addr = format!("{}:{}", h, port);
    match TcpListener::bind(&addr) {
        Ok(listener) => {
            let handle = next_handle();
            let mut map = LISTENERS.lock().unwrap();
            if map.is_none() {
                *map = Some(HashMap::new());
            }
            map.as_mut().unwrap().insert(handle, listener);
            handle
        }
        Err(_) => -1,
    }
}

/// Accept a pending connection. Returns client socket handle, or -1 on error.
/// This is a blocking call.
#[no_mangle]
pub extern "C" fn __net_tcp_accept(server_handle: i64) -> i64 {
    let mut map = LISTENERS.lock().unwrap();
    if map.is_none() {
        return -1;
    }
    let listeners = map.as_mut().unwrap();
    if let Some(listener) = listeners.get_mut(&server_handle) {
        match listener.accept() {
            Ok((stream, _addr)) => {
                let handle = next_handle();
                let mut smap = SOCKETS.lock().unwrap();
                if smap.is_none() {
                    *smap = Some(HashMap::new());
                }
                smap.as_mut().unwrap().insert(handle, stream);
                handle
            }
            Err(_) => -1,
        }
    } else {
        -1
    }
}

/// Close server handle and remove from registry.
#[no_mangle]
pub extern "C" fn __net_tcp_server_close(server_handle: i64) {
    let mut map = LISTENERS.lock().unwrap();
    if let Some(ref mut listeners) = *map {
        listeners.remove(&server_handle);
    }
}

// ============================================================
// UDP Socket Operations
// ============================================================

/// Create a new UDP socket. Returns a positive handle, or -1 on error.
#[no_mangle]
pub extern "C" fn __net_udp_socket() -> i64 {
    match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => {
            let handle = next_handle();
            let mut map = UDP_SOCKETS.lock().unwrap();
            if map.is_none() {
                *map = Some(HashMap::new());
            }
            map.as_mut().unwrap().insert(
                handle,
                UdpState {
                    socket,
                    sender_host: String::new(),
                    sender_port: 0,
                },
            );
            handle
        }
        Err(_) => -1,
    }
}

/// Bind the UDP socket to `host:port`. Returns 0 on success, -1 on error.
#[no_mangle]
pub extern "C" fn __net_udp_bind(handle: i64, host: *const c_char, port: i64) -> i64 {
    let h = unsafe { cstr_to_str(host) };
    let addr = format!("{}:{}", h, port);
    let mut map = UDP_SOCKETS.lock().unwrap();
    if map.is_none() {
        return -1;
    }
    let socks = map.as_mut().unwrap();
    if let Some(state) = socks.get_mut(&handle) {
        // Re-bind: create new socket, replace old one
        match UdpSocket::bind(&addr) {
            Ok(new_socket) => {
                state.socket = new_socket;
                0
            }
            Err(_) => -1,
        }
    } else {
        -1
    }
}

/// Send data to `host:port`. Returns bytes sent, or -1 on error.
#[no_mangle]
pub extern "C" fn __net_udp_send_to(
    handle: i64,
    host: *const c_char,
    port: i64,
    data: *const c_char,
) -> i64 {
    let h = unsafe { cstr_to_str(host) };
    let d = unsafe { cstr_to_str(data) };
    let addr = format!("{}:{}", h, port);
    let mut map = UDP_SOCKETS.lock().unwrap();
    if map.is_none() {
        return -1;
    }
    let socks = map.as_mut().unwrap();
    if let Some(state) = socks.get_mut(&handle) {
        match state.socket.send_to(d.as_bytes(), &addr) {
            Ok(n) => n as i64,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

/// Receive a datagram. Returns the data string (may be empty).
/// Sender host/port are stored per-socket and retrieved via
/// __net_udp_sender_host / __net_udp_sender_port.
#[no_mangle]
pub extern "C" fn __net_udp_recv_from(handle: i64, max_bytes: i64) -> *mut c_char {
    let mut map = UDP_SOCKETS.lock().unwrap();
    if map.is_none() {
        return unsafe { str_to_heap("") };
    }
    let socks = map.as_mut().unwrap();
    if let Some(state) = socks.get_mut(&handle) {
        let mut buf = vec![0u8; max_bytes as usize];
        match state.socket.recv_from(&mut buf) {
            Ok((n, src)) => {
                state.sender_host = src.ip().to_string();
                state.sender_port = src.port() as i64;
                if n == 0 {
                    unsafe { str_to_heap("") }
                } else {
                    unsafe { str_to_heap(&String::from_utf8_lossy(&buf[..n])) }
                }
            }
            Err(_) => unsafe { str_to_heap("") },
        }
    } else {
        unsafe { str_to_heap("") }
    }
}

/// Return the host of the last received datagram.
#[no_mangle]
pub extern "C" fn __net_udp_sender_host(handle: i64) -> *mut c_char {
    let map = UDP_SOCKETS.lock().unwrap();
    if map.is_none() {
        return unsafe { str_to_heap("") };
    }
    let socks = map.as_ref().unwrap();
    if let Some(state) = socks.get(&handle) {
        unsafe { str_to_heap(&state.sender_host) }
    } else {
        unsafe { str_to_heap("") }
    }
}

/// Return the port of the last received datagram.
#[no_mangle]
pub extern "C" fn __net_udp_sender_port(handle: i64) -> i64 {
    let map = UDP_SOCKETS.lock().unwrap();
    if map.is_none() {
        return -1;
    }
    let socks = map.as_ref().unwrap();
    if let Some(state) = socks.get(&handle) {
        state.sender_port
    } else {
        -1
    }
}

/// Close the UDP socket and remove from registry.
#[no_mangle]
pub extern "C" fn __net_udp_close(handle: i64) {
    let mut map = UDP_SOCKETS.lock().unwrap();
    if let Some(ref mut socks) = *map {
        socks.remove(&handle);
    }
}
