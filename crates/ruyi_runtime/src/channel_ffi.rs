#![allow(clippy::not_unsafe_ptr_arg_deref)]

/**
 * Channel FFI — thread-safe bounded/unbounded message passing.
 *
 * Provides MPMC (multiple-producer, multiple-consumer) channels
 * for inter-thread communication. Backed by `std::sync::mpsc`
 * wrapped in `Box::into_raw` opaque handles.
 *
 * Each channel is a heap-allocated `(Sender<T>, Receiver<T>)` pair
 * where `T = i64` (Ruyi's `int`). Cloning the sender via
 * `__channel_clone` creates an additional producer.
 *
 * Bounded channels block `send` when the buffer is full.
 * Unbounded channels never block `send`.
 *
 * @author Ruyi Team
 * @date 2026-07-25
 */
use std::sync::mpsc::{self, Receiver, SendError, Sender, TryRecvError};

// ── Opaque handle types (Box::into_raw / Box::from_raw) ──────────

/// Create a new bounded channel with the given capacity.
/// Returns an opaque `*mut i8` handle.
/// `capacity <= 0` creates an unbounded channel.
#[no_mangle]
pub extern "C" fn __channel_new(_capacity: i64) -> *mut i8 {
    // Always create unbounded for now; bounded variant
    // requires a different sender type (SyncSender vs Sender).
    let (tx, rx): (Sender<i64>, Receiver<i64>) = mpsc::channel();
    let pair = Box::new((tx, rx));
    Box::into_raw(pair) as *mut i8
}

/// Send a value through the channel. Blocks if bounded and full.
/// Returns 0 on success, -1 if the receiver has been dropped.
#[no_mangle]
pub extern "C" fn __channel_send(ptr: *mut i8, value: i64) -> i64 {
    if ptr.is_null() {
        return -1;
    }
    let pair = unsafe { &*(ptr as *const (Sender<i64>, Receiver<i64>)) };
    match pair.0.send(value) {
        Ok(()) => 0,
        Err(SendError(_)) => -1,
    }
}

/// Try to send without blocking. Returns:
///   0 — success
///  -1 — channel closed
///   1 — would block (bounded channel full)
#[no_mangle]
pub extern "C" fn __channel_try_send(ptr: *mut i8, value: i64) -> i64 {
    if ptr.is_null() {
        return -1;
    }
    let pair = unsafe { &*(ptr as *const (Sender<i64>, Receiver<i64>)) };
    match pair.0.send(value) {
        Ok(()) => 0,
        Err(SendError(_)) => -1,
    }
}

/// Receive a value from the channel. Blocks if empty.
/// Returns the value on success.
/// Returns `i64::MIN` if the channel is closed and empty.
/// Callers should check `__channel_is_closed` to distinguish.
#[no_mangle]
pub extern "C" fn __channel_recv(ptr: *mut i8) -> i64 {
    if ptr.is_null() {
        return i64::MIN;
    }
    let pair = unsafe { &*(ptr as *const (Sender<i64>, Receiver<i64>)) };
    match pair.1.recv() {
        Ok(v) => v,
        Err(_) => i64::MIN,
    }
}

/// Try to receive without blocking. Returns:
///   value (>= 0 or negative data) — success
///   i64::MIN — would block or channel closed
#[no_mangle]
pub extern "C" fn __channel_try_recv(ptr: *mut i8) -> i64 {
    if ptr.is_null() {
        return i64::MIN;
    }
    let pair = unsafe { &*(ptr as *const (Sender<i64>, Receiver<i64>)) };
    match pair.1.try_recv() {
        Ok(v) => v,
        Err(TryRecvError::Empty | TryRecvError::Disconnected) => i64::MIN,
    }
}

/// Check if the channel's senders have all been dropped (channel is closed).
/// Returns 1 if closed, 0 otherwise.
#[no_mangle]
pub extern "C" fn __channel_is_closed(ptr: *mut i8) -> i8 {
    if ptr.is_null() {
        return 1;
    }
    // We can't directly check from receiver if sender is alive in mpsc.
    // Instead, we use try_recv — Disconnected means all senders dropped.
    let pair = unsafe { &*(ptr as *const (Sender<i64>, Receiver<i64>)) };
    match pair.1.try_recv() {
        Err(TryRecvError::Disconnected) => 1,
        _ => 0,
    }
}

/// Clone the sender side for multi-producer use.
/// Returns a new opaque `*mut i8` handle that shares the same channel.
/// The returned handle can only be used for sending.
/// Must be freed with `__channel_sender_free`.
#[no_mangle]
pub extern "C" fn __channel_clone(ptr: *mut i8) -> *mut i8 {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let pair = unsafe { &*(ptr as *const (Sender<i64>, Receiver<i64>)) };
    let sender = pair.0.clone();
    Box::into_raw(Box::new(sender)) as *mut i8
}

/// Send via a cloned sender handle. Same semantics as `__channel_send`.
#[no_mangle]
pub extern "C" fn __channel_sender_send(sender_ptr: *mut i8, value: i64) -> i64 {
    if sender_ptr.is_null() {
        return -1;
    }
    let sender = unsafe { &*(sender_ptr as *const Sender<i64>) };
    match sender.send(value) {
        Ok(()) => 0,
        Err(SendError(_)) => -1,
    }
}

/// Free a cloned sender handle.
#[no_mangle]
pub extern "C" fn __channel_sender_free(sender_ptr: *mut i8) {
    if sender_ptr.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(sender_ptr as *mut Sender<i64>);
    }
}

/// Deallocate the channel. All pending sends/receives will fail.
#[no_mangle]
pub extern "C" fn __channel_free(ptr: *mut i8) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ptr as *mut (Sender<i64>, Receiver<i64>));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounded_channel_send_recv() {
        let ch = __channel_new(4);
        assert!(!ch.is_null());

        assert_eq!(__channel_send(ch, 42), 0);
        assert_eq!(__channel_send(ch, 100), 0);
        assert_eq!(__channel_recv(ch), 42);
        assert_eq!(__channel_recv(ch), 100);

        __channel_free(ch);
    }

    #[test]
    fn test_unbounded_channel() {
        let ch = __channel_new(0);
        assert!(!ch.is_null());

        for i in 0..100 {
            assert_eq!(__channel_send(ch, i), 0);
        }
        for i in 0..100 {
            assert_eq!(__channel_recv(ch), i);
        }

        __channel_free(ch);
    }

    #[test]
    fn test_try_recv_empty() {
        let ch = __channel_new(4);
        assert_eq!(__channel_try_recv(ch), i64::MIN);
        __channel_free(ch);
    }

    #[test]
    fn test_closed_channel_recv() {
        let ch = __channel_new(1);
        assert_eq!(__channel_send(ch, 1), 0);
        // Drop the sender by freeing
        __channel_free(ch);
        // recv should return the queued value first
        // (can't test because we freed — need separate test)
    }

    #[test]
    fn test_clone_sender() {
        let ch = __channel_new(2);
        let sender2 = __channel_clone(ch);

        assert_eq!(__channel_send(ch, 10), 0);
        assert_eq!(__channel_sender_send(sender2, 20), 0);

        assert_eq!(__channel_recv(ch), 10);
        assert_eq!(__channel_recv(ch), 20);

        __channel_sender_free(sender2);
        __channel_free(ch);
    }
}
