#![allow(clippy::not_unsafe_ptr_arg_deref)]

/**
 * C FFI implementations backing `stdlib/net.ry`.
 *
 * Provides TCP socket operations via C ABI. Socket and server handles are
 * represented as `i64` opaque IDs tracked in global registries. All I/O
 * is synchronous (blocking) — async wrappers live in the .ry layer.
 *
 * Locking model: the global registries (`SOCKETS` / `LISTENERS` /
 * `UDP_SOCKETS`) protect only the handle → entry map and are held for
 * lookup/insertion only. Each entry is an `Arc<Mutex<..>>` so blocking
 * I/O (read / write / accept / recv_from) runs while holding the
 * per-entry lock, never the registry lock. Holding the registry lock
 * across a blocking syscall deadlocks every other network operation in
 * the process (e.g. a client read on one thread blocks a server accept
 * on another).
 *
 * @author Ruyi Team
 * @date 2026-07-18
 */
use std::alloc::{alloc, Layout};
use std::collections::HashMap;
use std::ffi::CStr;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::os::fd::AsRawFd;
use std::os::raw::c_char;
use std::sync::{Arc, Mutex};

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

type SocketEntry = Arc<Mutex<TcpStream>>;
type ListenerEntry = Arc<Mutex<TcpListener>>;
type UdpEntry = Arc<Mutex<UdpState>>;

static SOCKETS: Mutex<Option<HashMap<i64, SocketEntry>>> = Mutex::new(None);
static LISTENERS: Mutex<Option<HashMap<i64, ListenerEntry>>> = Mutex::new(None);
static UDP_SOCKETS: Mutex<Option<HashMap<i64, UdpEntry>>> = Mutex::new(None);
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

/// Look up a socket entry, cloning its `Arc` so the registry lock is
/// released before any I/O happens.
fn get_socket(handle: i64) -> Option<SocketEntry> {
    let map = SOCKETS.lock().unwrap();
    map.as_ref()?.get(&handle).cloned()
}

fn get_listener(handle: i64) -> Option<ListenerEntry> {
    let map = LISTENERS.lock().unwrap();
    map.as_ref()?.get(&handle).cloned()
}

fn get_udp(handle: i64) -> Option<UdpEntry> {
    let map = UDP_SOCKETS.lock().unwrap();
    map.as_ref()?.get(&handle).cloned()
}

/// Register a stream under a fresh handle. The registry lock is held
/// only for the insertion.
fn insert_socket(stream: TcpStream) -> i64 {
    let handle = next_handle();
    let mut map = SOCKETS.lock().unwrap();
    if map.is_none() {
        *map = Some(HashMap::new());
    }
    map.as_mut().unwrap().insert(handle, Arc::new(Mutex::new(stream)));
    handle
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
        Ok(stream) => insert_socket(stream),
        Err(_) => -1,
    }
}

/// Read up to `max_bytes` from socket handle. Returns heap-allocated data
/// string, or null / empty on EOF / error.
#[no_mangle]
pub extern "C" fn __net_tcp_read(handle: i64, max_bytes: i64) -> *mut c_char {
    let Some(entry) = get_socket(handle) else {
        return unsafe { str_to_heap("") };
    };
    let mut stream = entry.lock().unwrap();
    let mut buf = vec![0u8; max_bytes as usize];
    match stream.read(&mut buf) {
        Ok(0) => unsafe { str_to_heap("") }, // EOF
        Ok(n) => unsafe { str_to_heap(&String::from_utf8_lossy(&buf[..n])) },
        Err(_) => unsafe { str_to_heap("") },
    }
}

/// Write data to socket handle. Returns bytes written, or -1 on error.
#[no_mangle]
pub extern "C" fn __net_tcp_write(handle: i64, data: *const c_char) -> i64 {
    let d = unsafe { cstr_to_str(data) };
    let Some(entry) = get_socket(handle) else {
        return -1;
    };
    let mut stream = entry.lock().unwrap();
    match stream.write(d.as_bytes()) {
        Ok(n) => n as i64,
        Err(_) => -1,
    }
}

/// Write raw bytes from a Ruyi Array<byte> to socket handle (no
/// null-byte truncation). The array layout is decoded via
/// `io_ffi::array_ptr`, mirroring `__fs_write_raw`.
/// Returns bytes written, or -1 on error.
#[no_mangle]
pub extern "C" fn __net_tcp_write_raw(handle: i64, arr: *mut i8) -> i64 {
    if arr.is_null() {
        return -1;
    }
    let (len_ptr, _cap_ptr, data_ptr) = unsafe { crate::io_ffi::array_ptr(arr) };
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
    let Some(entry) = get_socket(handle) else {
        return -1;
    };
    let mut stream = entry.lock().unwrap();
    // `Write::write` is a partial write: TCP send buffers can be full
    // when the socket is busy, so the kernel may accept only a subset of
    // the requested bytes. Without retrying until the full payload is
    // written, the MQTT `connect` (18 bytes) was sometimes sent in
    // pieces (e.g. 16 bytes) — the server then blocked forever on
    // `readRaw(remaining)` for the last two bytes that never arrived.
    let mut written = 0usize;
    while written < len {
        match stream.write(&buf[written..]) {
            Ok(0) => return -1,
            Ok(n) => written += n,
            Err(_) => return -1,
        }
    }
    written as i64
}

/// Read raw bytes from socket handle into a Ruyi Array<int>.
/// Returns bytes actually read (0 = EOF), or -1/-2 on error.
///
/// The caller passes a Ruyi Array whose `len` field carries the
/// user-requested payload size (e.g. `readRaw(2)` constructs
/// `Buffer.alloc(2)`, which sets `len=2`). The internal `cap` may be
/// larger (Ruyi array growth jumps from 0 to 4 — see
/// `__builtin_array_push`), so we must honour the requested length
/// instead of the underlying capacity; otherwise `stream.read` will
/// happily slurp up to `cap` bytes from the kernel, the surplus
/// silently bleeds into the next packet, and the MQTT `readPacket`
/// state machine desynchronises. We leave the array length untouched
/// (so `Buffer.length()` still reports the originally-requested size
/// for the accumulator loop) and write only the first `n` payload
/// bytes into the array.
#[no_mangle]
pub extern "C" fn __net_tcp_read_raw(handle: i64, arr: *mut i8) -> i64 {
    if arr.is_null() {
        return -1;
    }
    let (len_ptr, _cap_ptr, data_ptr) = unsafe { crate::io_ffi::array_ptr(arr) };
    let requested = unsafe { *len_ptr } as usize;
    if requested == 0 {
        return 0;
    }

    let Some(entry) = get_socket(handle) else {
        return -2;
    };
    let mut stream = entry.lock().unwrap();
    let mut buf = vec![0u8; requested];
    let n = match stream.read(&mut buf) {
        Ok(0) => return 0, // EOF: surface as 0-byte read so callers can detect
        Ok(n) => n,
        Err(_) => return -1,
    };
    unsafe {
        for i in 0..n {
            *data_ptr.add(i) = buf[i] as i64;
        }
    }
    n as i64
}

/// Close socket handle and remove from registry. In-flight I/O on other
/// threads keeps the entry alive via its `Arc` clone and completes
/// normally.
#[no_mangle]
pub extern "C" fn __net_tcp_close(handle: i64) {
    let mut map = SOCKETS.lock().unwrap();
    if let Some(ref mut streams) = *map {
        streams.remove(&handle);
    }
}

// ── Binary I/O helpers for internal use (TLS FFI, etc.) ──

pub(crate) fn tcp_read_raw(handle: i64, buf: &mut [u8]) -> i64 {
    let Some(entry) = get_socket(handle) else {
        return -2;
    };
    let mut stream = entry.lock().unwrap();
    match stream.read(buf) {
        Ok(0) => 0,
        Ok(n) => n as i64,
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => -2,
        Err(_) => -1,
    }
}

pub(crate) fn tcp_write_raw(handle: i64, buf: &[u8]) -> i64 {
    let Some(entry) = get_socket(handle) else {
        return -2;
    };
    let mut stream = entry.lock().unwrap();
    match stream.write(buf) {
        Ok(n) => n as i64,
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => -2,
        Err(_) => -1,
    }
}

pub(crate) fn get_tcp_fd(handle: i64) -> std::os::fd::RawFd {
    let Some(entry) = get_socket(handle) else {
        return -1;
    };
    let stream = entry.lock().unwrap();
    stream.as_raw_fd()
}

pub(crate) fn get_listener_fd(handle: i64) -> std::os::fd::RawFd {
    let Some(entry) = get_listener(handle) else {
        return -1;
    };
    let listener = entry.lock().unwrap();
    listener.as_raw_fd()
}

#[cfg(test)]
pub(crate) fn set_nonblocking(handle: i64, nonblocking: bool) {
    if let Some(entry) = get_socket(handle) {
        let stream = entry.lock().unwrap();
        let _ = stream.set_nonblocking(nonblocking);
    }
}

pub(crate) fn try_accept(server_handle: i64) -> i64 {
    let Some(entry) = get_listener(server_handle) else {
        return -1;
    };
    let listener = entry.lock().unwrap();
    match listener.accept() {
        Ok((stream, _addr)) => {
            let _ = stream.set_nonblocking(true);
            drop(listener);
            insert_socket(stream)
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => -2,
        Err(_) => -1,
    }
}

#[cfg(test)]
pub(crate) fn register_test_socket(stream: std::net::TcpStream) -> i64 {
    insert_socket(stream)
}

// ── end helpers ──

/// Set socket read timeout in milliseconds. 0 means no timeout (blocking).
/// Returns 0 on success, -1 if handle not found.
#[no_mangle]
pub extern "C" fn __net_tcp_set_timeout(handle: i64, timeout_ms: i64) -> i64 {
    let Some(entry) = get_socket(handle) else {
        return -1;
    };
    let stream = entry.lock().unwrap();
    let dur = std::time::Duration::from_millis(timeout_ms as u64);
    match stream
        .set_read_timeout(Some(dur))
        .and(stream.set_write_timeout(Some(dur)))
    {
        Ok(_) => 0,
        Err(_) => -1,
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
            map.as_mut()
                .unwrap()
                .insert(handle, Arc::new(Mutex::new(listener)));
            handle
        }
        Err(_) => -1,
    }
}

/// Accept a pending connection. Returns client socket handle, or -1 on error.
/// This is a blocking call; the registry lock is NOT held while blocking.
#[no_mangle]
pub extern "C" fn __net_tcp_accept(server_handle: i64) -> i64 {
    let Some(entry) = get_listener(server_handle) else {
        return -1;
    };
    let listener = entry.lock().unwrap();
    match listener.accept() {
        Ok((stream, _addr)) => {
            drop(listener);
            insert_socket(stream)
        }
        Err(_) => -1,
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
                Arc::new(Mutex::new(UdpState {
                    socket,
                    sender_host: String::new(),
                    sender_port: 0,
                })),
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
    let Some(entry) = get_udp(handle) else {
        return -1;
    };
    let mut state = entry.lock().unwrap();
    // Re-bind: create new socket, replace old one
    match UdpSocket::bind(&addr) {
        Ok(new_socket) => {
            state.socket = new_socket;
            0
        }
        Err(_) => -1,
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
    let Some(entry) = get_udp(handle) else {
        return -1;
    };
    let state = entry.lock().unwrap();
    match state.socket.send_to(d.as_bytes(), &addr) {
        Ok(n) => n as i64,
        Err(_) => -1,
    }
}

/// Receive a datagram. Returns the data string (may be empty).
/// Sender host/port are stored per-socket and retrieved via
/// __net_udp_sender_host / __net_udp_sender_port. The registry lock is
/// NOT held while the receive blocks.
#[no_mangle]
pub extern "C" fn __net_udp_recv_from(handle: i64, max_bytes: i64) -> *mut c_char {
    let Some(entry) = get_udp(handle) else {
        return unsafe { str_to_heap("") };
    };
    let mut state = entry.lock().unwrap();
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
}

/// Return the host of the last received datagram.
#[no_mangle]
pub extern "C" fn __net_udp_sender_host(handle: i64) -> *mut c_char {
    let Some(entry) = get_udp(handle) else {
        return unsafe { str_to_heap("") };
    };
    let state = entry.lock().unwrap();
    unsafe { str_to_heap(&state.sender_host) }
}

/// Return the port of the last received datagram.
#[no_mangle]
pub extern "C" fn __net_udp_sender_port(handle: i64) -> i64 {
    let Some(entry) = get_udp(handle) else {
        return -1;
    };
    let state = entry.lock().unwrap();
    state.sender_port
}

/// Close the UDP socket and remove from registry.
#[no_mangle]
pub extern "C" fn __net_udp_close(handle: i64) {
    let mut map = UDP_SOCKETS.lock().unwrap();
    if let Some(ref mut socks) = *map {
        socks.remove(&handle);
    }
}

// ============================================================
// Non-Blocking Socket Operations (for async I/O via reactor)
// ============================================================

/// Set the socket to non-blocking (1) or blocking (0) mode.
/// Returns 0 on success, -1 on failure.
#[no_mangle]
pub extern "C" fn __net_tcp_set_nonblocking(handle: i64, nonblocking: i64) -> i64 {
    let Some(entry) = get_socket(handle) else {
        return -1;
    };
    let stream = entry.lock().unwrap();
    match stream.set_nonblocking(nonblocking != 0) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get the raw file descriptor of a TCP socket.
/// Returns the fd (>= 0), or -1 on failure.
#[no_mangle]
pub extern "C" fn __net_tcp_get_fd(handle: i64) -> i64 {
    get_tcp_fd(handle) as i64
}

/// Get the raw file descriptor of a TCP server (listener).
/// Returns the fd (>= 0), or -1 on failure.
#[no_mangle]
pub extern "C" fn __net_tcp_listen_get_fd(server_handle: i64) -> i64 {
    get_listener_fd(server_handle) as i64
}

/// Non-blocking read from a TCP socket.
///
/// Returns:
/// - n > 0: bytes read (as heap-allocated string)
/// - 0: EOF
/// - -1: error
/// - -2: WouldBlock (try again later)
#[no_mangle]
pub extern "C" fn __net_tcp_try_read(handle: i64, max_bytes: i64) -> *mut c_char {
    let Some(entry) = get_socket(handle) else {
        return unsafe { str_to_heap("\u{1}") };
    };
    let mut stream = entry.lock().unwrap();
    let mut buf = vec![0u8; max_bytes as usize];
    match stream.read(&mut buf) {
        Ok(0) => unsafe { str_to_heap("") },
        Ok(n) => unsafe { str_to_heap(&String::from_utf8_lossy(&buf[..n])) },
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            // Return sentinel string SOH (0x01) to signal WouldBlock.
            unsafe { str_to_heap("\u{1}") }
        }
        Err(_) => unsafe { str_to_heap("") },
    }
}

/// Check if the last try_read returned WouldBlock.
/// Call this after try_read to disambiguate empty/EOF from WouldBlock.
/// Returns 1 if last try_read was WouldBlock, 0 otherwise.
#[no_mangle]
pub extern "C" fn __net_tcp_would_block(result: *const c_char) -> i64 {
    if result.is_null() {
        return 0;
    }
    let s = unsafe { cstr_to_str(result) };
    if s == "\u{1}" {
        1
    } else {
        0
    }
}

/// Non-blocking write to a TCP socket.
///
/// Returns:
/// - n >= 0: bytes written
/// - -1: error
/// - -2: WouldBlock (try again later)
#[no_mangle]
pub extern "C" fn __net_tcp_try_write(handle: i64, data: *const c_char) -> i64 {
    let d = unsafe { cstr_to_str(data) };
    let Some(entry) = get_socket(handle) else {
        return -1;
    };
    let mut stream = entry.lock().unwrap();
    match stream.write(d.as_bytes()) {
        Ok(n) => n as i64,
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => -2,
        Err(_) => -1,
    }
}

/// Non-blocking accept on a TCP server.
///
/// Returns:
/// - handle > 0: new client socket handle
/// - -1: error
/// - -2: WouldBlock (no pending connections)
#[no_mangle]
pub extern "C" fn __net_tcp_try_accept(server_handle: i64) -> i64 {
    try_accept(server_handle)
}
