#![allow(clippy::not_unsafe_ptr_arg_deref)]

/**
 * TLS FFI — HTTPS / secure sockets via rustls.
 *
 * Client (`__tls_*`) and server (`__tls_server_*`) share the same
 * I/O helpers; the connection type differs (ClientConnection vs
 * ServerConnection).
 *
 * @author Ruyi Team
 * @date 2026-07-24
 */
use std::ffi::{CStr, CString};
use std::io::{self, Read, Write};
use std::os::raw::c_char;
use std::sync::{Arc, OnceLock};

use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ClientConfig, ClientConnection, Connection, ServerConfig, ServerConnection};

use crate::net_ffi::{tcp_read_raw, tcp_write_raw};

// ── crypto / root certs (lazy init) ──────────────────────────

fn init_crypto() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .expect("rustls crypto provider init failed");
    });
}

fn root_store() -> &'static rustls::RootCertStore {
    static STORE: OnceLock<rustls::RootCertStore> = OnceLock::new();
    STORE.get_or_init(|| {
        let mut store = rustls::RootCertStore::empty();
        store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        store
    })
}

// ── PEM parsing ──────────────────────────────────────────────

fn parse_certs(pem: &str) -> Result<Vec<CertificateDer<'static>>, String> {
    let mut reader = std::io::BufReader::new(pem.as_bytes());
    rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("cert parse: {}", e))
}

fn parse_key(pem: &str) -> Result<PrivateKeyDer<'static>, String> {
    let mut reader = std::io::BufReader::new(pem.as_bytes());
    rustls_pemfile::private_key(&mut reader)
        .map_err(|e| format!("key parse: {}", e))?
        .ok_or_else(|| "no private key found".to_string())
}

// ── socket I/O ───────────────────────────────────────────────

fn tcp_read(socket: i64, buf: &mut [u8]) -> io::Result<usize> {
    let n = tcp_read_raw(socket, buf);
    match n {
        n if n > 0 => Ok(n as usize),
        0 => Ok(0),
        _ => Err(io::Error::new(io::ErrorKind::ConnectionAborted, "tcp read")),
    }
}

fn tcp_write(socket: i64, buf: &[u8]) -> io::Result<()> {
    if tcp_write_raw(socket, buf) >= 0 {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            "tcp write",
        ))
    }
}

/// Drain queued TLS output to the TCP socket.
fn tls_flush(conn: &mut Connection, socket: i64) -> io::Result<()> {
    while conn.wants_write() {
        let mut out = Vec::new();
        conn.write_tls(&mut out)?;
        if out.is_empty() {
            break;
        }
        tcp_write(socket, &out)?;
    }
    Ok(())
}

/// Blocking TLS handshake (shared between client & server).
fn handshake(conn: &mut Connection, socket: i64) -> io::Result<()> {
    while conn.is_handshaking() {
        if conn.wants_write() {
            let mut out = Vec::new();
            conn.write_tls(&mut out)?;
            tcp_write(socket, &out)?;
        }
        if conn.wants_read() {
            let mut buf = [0u8; 16384];
            let n = tcp_read(socket, &mut buf)?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "eof during handshake",
                ));
            }
            conn.read_tls(&mut io::Cursor::new(&buf[..n]))?;
            tls_flush(conn, socket)?;
            conn.process_new_packets()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        }
    }
    tls_flush(conn, socket)
}

// ── generic read / write helpers ─────────────────────────────

fn read_cstr(conn: &mut Connection, socket: i64, max_len: i64) -> *mut c_char {
    if max_len <= 0 {
        return std::ptr::null_mut();
    }
    let mut buf = vec![0u8; max_len as usize];

    let n = loop {
        match conn.reader().read(&mut buf) {
            Ok(n) => break n as i64,
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                if !conn.wants_read() {
                    break 0;
                }
                let mut tls_buf = [0u8; 16384];
                let n = match tcp_read(socket, &mut tls_buf) {
                    Ok(n) => n,
                    Err(_) => break -1,
                };
                if n == 0 {
                    break 0;
                }
                if conn.read_tls(&mut io::Cursor::new(&tls_buf[..n])).is_err() {
                    break -1;
                }
                if tls_flush(conn, socket).is_err() {
                    break -1;
                }
                if conn
                    .process_new_packets()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
                    .is_err()
                {
                    break -1;
                }
            }
            Err(_) => break -1,
        }
    };

    if n <= 0 {
        return std::ptr::null_mut();
    }
    let s = String::from_utf8_lossy(&buf[..n as usize]).into_owned();
    CString::new(s).unwrap_or_default().into_raw()
}

fn write_cstr(conn: &mut Connection, socket: i64, data: *const c_char) -> i64 {
    if data.is_null() {
        return -1;
    }
    let bytes = unsafe { CStr::from_ptr(data) }.to_bytes();
    if bytes.is_empty() {
        return 0;
    }
    if conn.writer().write_all(bytes).is_err() {
        return -1;
    }
    if tls_flush(conn, socket).is_err() {
        return -1;
    }
    if conn.writer().flush().is_err() {
        return -1;
    }
    bytes.len() as i64
}

fn close_session(conn: &mut Connection, socket: i64) {
    conn.send_close_notify();
    let _ = tls_flush(conn, socket);
    crate::net_ffi::__net_tcp_close(socket);
}

// ── session types ────────────────────────────────────────────

struct Session {
    conn: Connection,
    socket: i64,
}

// ============================================================
// Client FFI
// ============================================================

#[no_mangle]
pub extern "C" fn __tls_connect(socket: i64, hostname: *const c_char) -> *mut i8 {
    init_crypto();
    let name = if hostname.is_null() {
        return std::ptr::null_mut();
    } else {
        match unsafe { CStr::from_ptr(hostname) }.to_str() {
            Ok(n) if !n.is_empty() => n,
            _ => return std::ptr::null_mut(),
        }
    };
    let server_name = match ServerName::try_from(name) {
        Ok(n) => n,
        Err(_) => return std::ptr::null_mut(),
    };
    let config = Arc::new(
        ClientConfig::builder()
            .with_root_certificates(root_store().clone())
            .with_no_client_auth(),
    );
    let conn = match ClientConnection::new(config, server_name) {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };
    let mut session = Box::new(Session {
        conn: Connection::Client(conn),
        socket,
    });
    if handshake(&mut session.conn, socket).is_err() {
        return std::ptr::null_mut();
    }
    Box::into_raw(session) as *mut i8
}

#[no_mangle]
pub extern "C" fn __tls_read_cstr(tls: *mut i8, max_len: i64) -> *mut c_char {
    if tls.is_null() {
        return std::ptr::null_mut();
    }
    let s = unsafe { &mut *(tls as *mut Session) };
    read_cstr(&mut s.conn, s.socket, max_len)
}

#[no_mangle]
pub extern "C" fn __tls_write(tls: *mut i8, data: *const c_char) -> i64 {
    if tls.is_null() {
        return -1;
    }
    let s = unsafe { &mut *(tls as *mut Session) };
    write_cstr(&mut s.conn, s.socket, data)
}

#[no_mangle]
pub extern "C" fn __tls_write_raw(tls: *mut i8, data: *const u8, len: i64) -> i64 {
    if tls.is_null() || data.is_null() || len <= 0 {
        return 0;
    }
    let bytes = unsafe { std::slice::from_raw_parts(data, len as usize) };
    let s = unsafe { &mut *(tls as *mut Session) };
    if s.conn.writer().write_all(bytes).is_err() {
        return -1;
    }
    if tls_flush(&mut s.conn, s.socket).is_err() {
        return -1;
    }
    if s.conn.writer().flush().is_err() {
        return -1;
    }
    len
}

/// Read raw bytes from a TLS session into a Ruyi Array<int>.
/// The array's capacity determines the max read size.
/// Returns bytes actually read (0 = EOF), or -1 on error.
fn read_buf(conn: &mut Connection, socket: i64, data_ptr: *mut i64, max_len: usize) -> i64 {
    let mut buf = vec![0u8; max_len];
    let n = loop {
        match conn.reader().read(&mut buf) {
            Ok(n) => break n as i64,
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                if !conn.wants_read() {
                    break 0;
                }
                let mut tls_buf = [0u8; 16384];
                let n = match tcp_read(socket, &mut tls_buf) {
                    Ok(n) => n,
                    Err(_) => break -1,
                };
                if n == 0 {
                    break 0;
                }
                if conn.read_tls(&mut io::Cursor::new(&tls_buf[..n])).is_err() {
                    break -1;
                }
                if tls_flush(conn, socket).is_err() {
                    break -1;
                }
                if conn
                    .process_new_packets()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
                    .is_err()
                {
                    break -1;
                }
            }
            Err(_) => break -1,
        }
    };
    if n <= 0 {
        return if n == 0 { 0 } else { -1 };
    }
    let n = n as usize;
    unsafe {
        for i in 0..n {
            *data_ptr.add(i) = buf[i] as i64;
        }
    }
    n as i64
}

#[no_mangle]
pub extern "C" fn __tls_read_raw(tls: *mut i8, arr: *mut i8) -> i64 {
    if tls.is_null() || arr.is_null() {
        return -1;
    }
    let (len_ptr, cap_ptr, data_ptr) = unsafe { crate::io_ffi::array_ptr(arr) };
    let cap = unsafe { *cap_ptr } as usize;
    if cap == 0 {
        return 0;
    }
    let s = unsafe { &mut *(tls as *mut Session) };
    let n = read_buf(&mut s.conn, s.socket, data_ptr, cap);
    if n > 0 {
        unsafe {
            *len_ptr = n;
        }
    }
    n
}

#[no_mangle]
pub extern "C" fn __tls_close(tls: *mut i8) {
    if tls.is_null() {
        return;
    }
    let s = unsafe { &mut *(tls as *mut Session) };
    close_session(&mut s.conn, s.socket);
}

#[no_mangle]
pub extern "C" fn __tls_free(tls: *mut i8) {
    if tls.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(tls as *mut Session);
    }
}

// ============================================================
// Server FFI
// ============================================================

/// Parse PEM cert + key → server config handle. Free with __tls_config_free.
#[no_mangle]
pub extern "C" fn __tls_server_config_new(
    cert_pem: *const c_char,
    key_pem: *const c_char,
) -> *mut i8 {
    init_crypto();
    if cert_pem.is_null() || key_pem.is_null() {
        return std::ptr::null_mut();
    }
    let cert_str = unsafe { CStr::from_ptr(cert_pem) }.to_str().unwrap_or("");
    let key_str = unsafe { CStr::from_ptr(key_pem) }.to_str().unwrap_or("");

    let certs = match parse_certs(cert_str) {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };
    let key = match parse_key(key_str) {
        Ok(k) => k,
        Err(_) => return std::ptr::null_mut(),
    };

    let config = match ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
    {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };

    Box::into_raw(Box::new(Arc::new(config))) as *mut i8
}

/// Accept a TLS connection on the given TCP socket using the server config.
/// Returns an opaque `*mut i8` session handle, or null on error.
#[no_mangle]
pub extern "C" fn __tls_server_accept(config: *mut i8, socket: i64) -> *mut i8 {
    if config.is_null() {
        return std::ptr::null_mut();
    }
    let cfg = unsafe { &*(config as *const Arc<ServerConfig>) }.clone();
    let conn = match ServerConnection::new(cfg) {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };
    let mut session = Box::new(Session {
        conn: Connection::Server(conn),
        socket,
    });
    if handshake(&mut session.conn, socket).is_err() {
        return std::ptr::null_mut();
    }
    Box::into_raw(session) as *mut i8
}

#[no_mangle]
pub extern "C" fn __tls_server_read_cstr(tls: *mut i8, max_len: i64) -> *mut c_char {
    if tls.is_null() {
        return std::ptr::null_mut();
    }
    let s = unsafe { &mut *(tls as *mut Session) };
    read_cstr(&mut s.conn, s.socket, max_len)
}

#[no_mangle]
pub extern "C" fn __tls_server_write(tls: *mut i8, data: *const c_char) -> i64 {
    if tls.is_null() {
        return -1;
    }
    let s = unsafe { &mut *(tls as *mut Session) };
    write_cstr(&mut s.conn, s.socket, data)
}

#[no_mangle]
pub extern "C" fn __tls_server_write_raw(tls: *mut i8, data: *const u8, len: i64) -> i64 {
    if tls.is_null() || data.is_null() || len <= 0 {
        return 0;
    }
    let bytes = unsafe { std::slice::from_raw_parts(data, len as usize) };
    let s = unsafe { &mut *(tls as *mut Session) };
    if s.conn.writer().write_all(bytes).is_err() {
        return -1;
    }
    if tls_flush(&mut s.conn, s.socket).is_err() {
        return -1;
    }
    if s.conn.writer().flush().is_err() {
        return -1;
    }
    len
}

#[no_mangle]
pub extern "C" fn __tls_server_read_raw(tls: *mut i8, arr: *mut i8) -> i64 {
    if tls.is_null() || arr.is_null() {
        return -1;
    }
    let (len_ptr, cap_ptr, data_ptr) = unsafe { crate::io_ffi::array_ptr(arr) };
    let cap = unsafe { *cap_ptr } as usize;
    if cap == 0 {
        return 0;
    }
    let s = unsafe { &mut *(tls as *mut Session) };
    let n = read_buf(&mut s.conn, s.socket, data_ptr, cap);
    if n > 0 {
        unsafe {
            *len_ptr = n;
        }
    }
    n
}

#[no_mangle]
pub extern "C" fn __tls_server_close(tls: *mut i8) {
    if tls.is_null() {
        return;
    }
    let s = unsafe { &mut *(tls as *mut Session) };
    close_session(&mut s.conn, s.socket);
}

#[no_mangle]
pub extern "C" fn __tls_server_free(tls: *mut i8) {
    if tls.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(tls as *mut Session);
    }
}

#[no_mangle]
pub extern "C" fn __tls_config_free(config: *mut i8) {
    if config.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(config as *mut Arc<ServerConfig>);
    }
}
