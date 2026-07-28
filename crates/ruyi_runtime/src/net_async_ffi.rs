//! Async I/O futures that integrate the Reactor with codegen's await protocol.
//!
//! Each future is a heap-allocated struct with a vtable (poll function pointer)
//! at offset 0, conforming to the `RawFuture` / `ruyi_await` contract in
//! `async_exports.rs`. The codegen creates these futures, awaits them, and
//! retrieves results — all through C FFI.
//!
//! ## Codegen Protocol
//!
//! ```text
//! // 1. Create future
//! data = __net_async_read(handle, max_bytes);
//! // 2. Codegen wraps in { poll_fn, data } and calls ruyi_await
//! //    → worker_loop polls → poll_fn(data, waker) called
//! //    → if WouldBlock: registers fd with Reactor, returns 0 (Pending)
//! //    → Reactor wakes task → poll_fn called again → returns 1 (Ready)
//! // 3. Get result
//! result_str = __net_async_read_result(data);
//! // 4. Free
//! __net_async_read_free(data);
//! ```
//!
//! @author Ruyi Team
//! @date 2026-07-25

use std::os::raw::c_char;

use crate::async_runtime::Waker;
use crate::reactor::GLOBAL_REACTOR;

use mio::Interest;

/// Poll function signature for codegen-compatible futures.
type PollFn = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;

// ==============================================================
// ReactorReadFuture — Async TCP read
// ==============================================================

/// State machine for an async TCP read that integrates with the Reactor.
#[repr(C)]
struct ReactorReadFuture {
    /// VTABLE — must be at offset 0 for `RawFuture::poll`.
    poll_fn: PollFn,
    /// Raw file descriptor for Reactor registration.
    raw_fd: i32,
    /// Socket handle in the global registry (for retry via net_ffi).
    socket_handle: i64,
    /// Maximum bytes to read.
    max_bytes: i64,
    /// Result pointer: null if not yet ready, otherwise heap-allocated string.
    result: *mut c_char,
    /// Reactor token (0 = not registered).
    reactor_token: u64,
    /// State: 0 = init, 1 = registered (waiting), 2 = done.
    state: u8,
    _pad: [u8; 7],
}

/// The poll function embedded as the vtable.
unsafe extern "C" fn reactor_read_poll(ptr: *mut u8, waker_ptr: *mut u8) -> i32 {
    let me: &mut ReactorReadFuture = unsafe { &mut *(ptr as *mut ReactorReadFuture) };
    let waker: &Waker = unsafe { &*(waker_ptr as *const Waker) };

    if me.state == 2 {
        return 1; // already done
    }

    // Try non-blocking read via the socket registry, using net_ffi's internal helpers.
    let mut buf = vec![0u8; me.max_bytes as usize];
    let n = crate::net_ffi::tcp_read_raw(me.socket_handle, &mut buf);

    match n {
        n if n > 0 => {
            // Success: copy data to heap string.
            let s = String::from_utf8_lossy(&buf[..n as usize]).to_string();
            me.result = unsafe { crate::io_ffi::str_to_heap(&s) };
            me.state = 2;
            if me.reactor_token > 0 {
                // Deregister from reactor.
                let _ = GLOBAL_REACTOR
                    .lock()
                    .unwrap()
                    .deregister(me.raw_fd, mio::Token(me.reactor_token as usize));
            }
            1
        }
        0 | -1 => {
            // EOF or error — signal completion.
            me.result = unsafe { crate::io_ffi::str_to_heap("") };
            me.state = 2;
            1
        }
        -2 => {
            // WouldBlock — register with Reactor and return Pending.
            if me.reactor_token == 0 {
                if let Ok(reactor) = GLOBAL_REACTOR.lock() {
                    if let Ok(token) =
                        reactor.register(me.raw_fd, Interest::READABLE, waker.clone())
                    {
                        me.reactor_token = token.0 as u64;
                    }
                }
            }
            me.state = 1;
            0
        }
        _ => {
            // Unknown error.
            me.result = unsafe { crate::io_ffi::str_to_heap("") };
            me.state = 2;
            1
        }
    }
}

/// Create an async TCP read future.
///
/// Returns a heap-allocated future handle. The first 8 bytes of the returned
/// pointer are a poll function — the codegen passes this directly to `ruyi_await`.
#[no_mangle]
pub extern "C" fn __net_async_read(handle: i64, max_bytes: i64) -> *mut u8 {
    let raw_fd = crate::net_ffi::get_tcp_fd(handle);
    let future = Box::new(ReactorReadFuture {
        poll_fn: reactor_read_poll,
        raw_fd,
        socket_handle: handle,
        max_bytes,
        result: std::ptr::null_mut(),
        reactor_token: 0,
        state: 0,
        _pad: [0u8; 7],
    });
    Box::into_raw(future) as *mut u8
}

/// Get the result string from a completed async read.
/// Returns null if the future hasn't completed yet.
#[no_mangle]
pub unsafe extern "C" fn __net_async_read_result(ptr: *mut u8) -> *mut c_char {
    let me = unsafe { &*(ptr as *const ReactorReadFuture) };
    me.result
}

/// Free an async read future and its associated resources.
#[no_mangle]
pub unsafe extern "C" fn __net_async_read_free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    let me: &mut ReactorReadFuture = unsafe { &mut *(ptr as *mut ReactorReadFuture) };
    if me.reactor_token > 0 {
        let _ = GLOBAL_REACTOR
            .lock()
            .unwrap()
            .deregister(me.raw_fd, mio::Token(me.reactor_token as usize));
    }
    if !me.result.is_null() {
        unsafe {
            let _ = std::ffi::CString::from_raw(me.result);
        }
    }
    unsafe {
        let _ = Box::from_raw(ptr as *mut ReactorReadFuture);
    }
}

// ==============================================================
// ReactorWriteFuture — Async TCP write
// ==============================================================

#[repr(C)]
struct ReactorWriteFuture {
    poll_fn: PollFn,
    raw_fd: i32,
    socket_handle: i64,
    data: *mut c_char, // heap-allocated C string to write
    result: i64,       // bytes written, -1 on error
    reactor_token: u64,
    state: u8,
    _pad: [u8; 7],
}

unsafe extern "C" fn reactor_write_poll(ptr: *mut u8, waker_ptr: *mut u8) -> i32 {
    let me: &mut ReactorWriteFuture = unsafe { &mut *(ptr as *mut ReactorWriteFuture) };
    let waker: &Waker = unsafe { &*(waker_ptr as *const Waker) };

    if me.state == 2 {
        return 1;
    }

    let s = unsafe { crate::io_ffi::cstr_to_str(me.data) };
    let n = crate::net_ffi::tcp_write_raw(me.socket_handle, s.as_bytes());

    match n {
        n if n >= 0 => {
            me.result = n;
            me.state = 2;
            if me.reactor_token > 0 {
                let _ = GLOBAL_REACTOR
                    .lock()
                    .unwrap()
                    .deregister(me.raw_fd, mio::Token(me.reactor_token as usize));
            }
            1
        }
        -2 => {
            // WouldBlock
            if me.reactor_token == 0 {
                if let Ok(reactor) = GLOBAL_REACTOR.lock() {
                    if let Ok(token) =
                        reactor.register(me.raw_fd, Interest::WRITABLE, waker.clone())
                    {
                        me.reactor_token = token.0 as u64;
                    }
                }
            }
            me.state = 1;
            0
        }
        _ => {
            me.result = -1;
            me.state = 2;
            1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn __net_async_write(handle: i64, data: *const c_char) -> *mut u8 {
    let raw_fd = crate::net_ffi::get_tcp_fd(handle);
    let s = unsafe { crate::io_ffi::cstr_to_str(data) };
    let future = Box::new(ReactorWriteFuture {
        poll_fn: reactor_write_poll,
        raw_fd,
        socket_handle: handle,
        data: unsafe { crate::io_ffi::str_to_heap(s) },
        result: -1,
        reactor_token: 0,
        state: 0,
        _pad: [0u8; 7],
    });
    Box::into_raw(future) as *mut u8
}

#[no_mangle]
pub unsafe extern "C" fn __net_async_write_result(ptr: *mut u8) -> i64 {
    let me = unsafe { &*(ptr as *const ReactorWriteFuture) };
    me.result
}

#[no_mangle]
pub unsafe extern "C" fn __net_async_write_free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    let me: &mut ReactorWriteFuture = unsafe { &mut *(ptr as *mut ReactorWriteFuture) };
    if me.reactor_token > 0 {
        let _ = GLOBAL_REACTOR
            .lock()
            .unwrap()
            .deregister(me.raw_fd, mio::Token(me.reactor_token as usize));
    }
    if !me.data.is_null() {
        unsafe {
            let _ = std::ffi::CString::from_raw(me.data);
        }
    }
    unsafe {
        let _ = Box::from_raw(ptr as *mut ReactorWriteFuture);
    }
}

// ==============================================================
// ReactorAcceptFuture — Async TCP accept
// ==============================================================

#[repr(C)]
struct ReactorAcceptFuture {
    poll_fn: PollFn,
    raw_fd: i32,
    server_handle: i64,
    result_handle: i64, // new client socket handle, -1 on error
    reactor_token: u64,
    state: u8,
    _pad: [u8; 7],
}

unsafe extern "C" fn reactor_accept_poll(ptr: *mut u8, waker_ptr: *mut u8) -> i32 {
    let me: &mut ReactorAcceptFuture = unsafe { &mut *(ptr as *mut ReactorAcceptFuture) };
    let waker: &Waker = unsafe { &*(waker_ptr as *const Waker) };

    if me.state == 2 {
        return 1;
    }

    let result = crate::net_ffi::try_accept(me.server_handle);
    match result {
        n if n >= 0 => {
            me.result_handle = n;
            me.state = 2;
            if me.reactor_token > 0 {
                let _ = GLOBAL_REACTOR
                    .lock()
                    .unwrap()
                    .deregister(me.raw_fd, mio::Token(me.reactor_token as usize));
            }
            1
        }
        -2 => {
            if me.reactor_token == 0 {
                if let Ok(reactor) = GLOBAL_REACTOR.lock() {
                    if let Ok(token) =
                        reactor.register(me.raw_fd, Interest::READABLE, waker.clone())
                    {
                        me.reactor_token = token.0 as u64;
                    }
                }
            }
            me.state = 1;
            0
        }
        _ => {
            me.result_handle = -1;
            me.state = 2;
            1
        }
    }
}

#[no_mangle]
pub extern "C" fn __net_async_accept(server_handle: i64) -> *mut u8 {
    let raw_fd = crate::net_ffi::get_listener_fd(server_handle);
    let future = Box::new(ReactorAcceptFuture {
        poll_fn: reactor_accept_poll,
        raw_fd,
        server_handle,
        result_handle: -1,
        reactor_token: 0,
        state: 0,
        _pad: [0u8; 7],
    });
    Box::into_raw(future) as *mut u8
}

#[no_mangle]
pub unsafe extern "C" fn __net_async_accept_result(ptr: *mut u8) -> i64 {
    let me = unsafe { &*(ptr as *const ReactorAcceptFuture) };
    me.result_handle
}

#[no_mangle]
pub unsafe extern "C" fn __net_async_accept_free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    let me: &mut ReactorAcceptFuture = unsafe { &mut *(ptr as *mut ReactorAcceptFuture) };
    if me.reactor_token > 0 {
        let _ = GLOBAL_REACTOR
            .lock()
            .unwrap()
            .deregister(me.raw_fd, mio::Token(me.reactor_token as usize));
    }
    unsafe {
        let _ = Box::from_raw(ptr as *mut ReactorAcceptFuture);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::async_runtime::{Scheduler, TaskId};
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;

    #[test]
    fn test_reactor_read_future_with_data() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let mut client = TcpStream::connect(addr).unwrap();
        let (server, _) = listener.accept().unwrap();
        server.set_nonblocking(true).unwrap();

        // Send data so first poll succeeds immediately.
        client.write_all(b"immediate").unwrap();
        client.flush().unwrap();
        std::thread::sleep(Duration::from_millis(20));

        let handle = crate::net_ffi::register_test_socket(server);
        crate::net_ffi::set_nonblocking(handle, true);
        let future_ptr = __net_async_read(handle, 1024);

        let scheduler = crate::async_runtime::SchedulerInner::new(1);
        let waker = Waker {
            scheduler: scheduler.clone(),
            worker_id: 0,
            task_id: TaskId(1),
        };
        let poll_fn: PollFn = unsafe {
            let vtable = std::ptr::read::<*mut u8>(future_ptr as *const *mut u8);
            std::mem::transmute(vtable)
        };
        let result = unsafe { poll_fn(future_ptr, &waker as *const Waker as *mut u8) };
        assert!(
            result == 1,
            "poll should return Ready when data is available"
        );
        let s = unsafe { crate::io_ffi::cstr_to_str(__net_async_read_result(future_ptr)) };
        assert_eq!(s, "immediate");

        unsafe {
            __net_async_read_free(future_ptr);
        }
    }

    #[test]
    fn test_reactor_read_future_would_block_then_ready() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        // Connect a client.
        let mut client = TcpStream::connect(addr).unwrap();
        let (server, _) = listener.accept().unwrap();
        server.set_nonblocking(true).unwrap();

        let handle = crate::net_ffi::register_test_socket(server);
        crate::net_ffi::set_nonblocking(handle, true);

        let future_ptr = __net_async_read(handle, 1024);

        let scheduler = crate::async_runtime::SchedulerInner::new(1);
        let waker = Waker {
            scheduler: scheduler.clone(),
            worker_id: 0,
            task_id: TaskId(2),
        };

        let poll_fn: PollFn = unsafe {
            let vtable = std::ptr::read::<*mut u8>(future_ptr as *const *mut u8);
            std::mem::transmute(vtable)
        };

        // First poll: no data, should return Pending and register with Reactor.
        let result1 = unsafe { poll_fn(future_ptr, &waker as *const Waker as *mut u8) };
        assert_eq!(result1, 0, "first poll should return Pending (WouldBlock)");

        // Write data from client side.
        client.write_all(b"hello reactor!").unwrap();
        client.flush().unwrap();

        // Wait a tiny bit for the data to arrive.
        std::thread::sleep(Duration::from_millis(50));

        // Poll the reactor to detect the ready fd.
        {
            let reactor = GLOBAL_REACTOR.lock().unwrap();
            reactor.poll(Some(Duration::from_millis(100))).ok();
        }

        // Second poll: data should be available now.
        let result2 = unsafe { poll_fn(future_ptr, &waker as *const Waker as *mut u8) };
        assert_eq!(result2, 1, "second poll should return Ready");

        // Check the result.
        let result_str = unsafe { __net_async_read_result(future_ptr) };
        assert!(!result_str.is_null());
        let s = unsafe { crate::io_ffi::cstr_to_str(result_str) };
        assert_eq!(s, "hello reactor!");

        unsafe {
            __net_async_read_free(future_ptr);
        }
    }
}
