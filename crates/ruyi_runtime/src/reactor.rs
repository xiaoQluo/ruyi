//! I/O Reactor — non-blocking event notification for the async runtime.
//!
//! Implements an epoll/kqueue-based reactor that bridges OS-level I/O readiness
//! to the async runtime's Waker mechanism. When a socket fd becomes readable or
//! writable, the reactor calls `waker.wake()` to re-schedule the awaiting task.
//!
//! ## Architecture
//!
//! ```text
//!   worker_loop()          Reactor             OS (epoll/kqueue)
//!   ─────────────          ───────             ─────────────────
//!   no tasks? ──────────▶ poll(timeout) ──────▶ epoll_wait(timeout)
//!                             │                       │
//!   next iter ◀─────── waker.wake() ◀───── ready events
//!        │
//!   polls future ────▶ Ready!
//! ```
//!
//! @author Ruyi Team
//! @date 2026-07-25

use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::io;
use std::os::fd::RawFd;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use crate::async_runtime::{TaskId, Waker};

/// Global reactor singleton, shared across all worker threads.
pub static GLOBAL_REACTOR: Lazy<Mutex<Reactor>> =
    Lazy::new(|| Mutex::new(Reactor::new().expect("failed to create I/O reactor")));

// ── Reactor ──────────────────────────────────────────────────

/// I/O Reactor that bridges OS-level readiness notification to async tasks.
///
/// Each registered fd is associated with a `Waker`. When the fd becomes ready,
/// the waker is called, pushing the task back onto the scheduler's run queue.
pub struct Reactor {
    /// The mio Poll instance (epoll on Linux, kqueue on macOS).
    /// Wrapped in Mutex because `Poll::poll()` takes `&mut self` in mio 1.x.
    poll: Mutex<Poll>,
    /// Pre-allocated event buffer for `poll()`.
    events: Mutex<Events>,
    /// Token → Waker map. The waker is called when the fd is ready.
    wakers: Mutex<HashMap<Token, Waker>>,
    /// Auto-incrementing token counter.
    next_token: AtomicUsize,
}

impl Reactor {
    /// Create a new reactor backed by the OS event notification system.
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            poll: Mutex::new(Poll::new()?),
            events: Mutex::new(Events::with_capacity(1024)),
            wakers: Mutex::new(HashMap::new()),
            next_token: AtomicUsize::new(1),
        })
    }

    /// Register a file descriptor for readiness notification.
    ///
    /// When the fd becomes ready for the given interest, the associated
    /// `Waker::wake()` will be called, re-scheduling the awaiting task.
    ///
    /// Returns the mio `Token` assigned to this registration.
    pub fn register(&self, fd: RawFd, interest: Interest, waker: Waker) -> io::Result<Token> {
        let token = Token(self.next_token.fetch_add(1, Ordering::SeqCst));

        let mut source = SourceFd(&fd);
        self.poll
            .lock()
            .unwrap()
            .registry()
            .register(&mut source, token, interest)?;

        self.wakers.lock().unwrap().insert(token, waker);
        Ok(token)
    }

    /// Re-register an existing token with new interests.
    pub fn reregister(&self, fd: RawFd, token: Token, interest: Interest) -> io::Result<()> {
        let mut source = SourceFd(&fd);
        self.poll
            .lock()
            .unwrap()
            .registry()
            .reregister(&mut source, token, interest)
    }

    /// Remove a registration. The waker is dropped.
    pub fn deregister(&self, fd: RawFd, token: Token) -> io::Result<()> {
        let mut source = SourceFd(&fd);
        self.poll
            .lock()
            .unwrap()
            .registry()
            .deregister(&mut source)?;
        self.wakers.lock().unwrap().remove(&token);
        Ok(())
    }

    /// Block until one or more registered fds become ready, or `timeout` expires.
    ///
    /// For each ready event, the associated waker is called, which pushes the
    /// task back onto the scheduler's run queue.
    ///
    /// Returns the number of events processed (tasks woken).
    pub fn poll(&self, timeout: Option<Duration>) -> io::Result<usize> {
        let mut events = self.events.lock().unwrap();
        events.clear();

        self.poll.lock().unwrap().poll(&mut events, timeout)?;

        let mut count: usize = 0;
        let wakers = self.wakers.lock().unwrap();
        for event in events.iter() {
            let token = event.token();
            if event.is_readable() || event.is_writable() {
                if let Some(waker) = wakers.get(&token) {
                    waker.wake();
                    count += 1;
                }
            }
        }
        Ok(count)
    }
}

// ── C FFI exports (for .ry codegen-generated futures) ────────

/// Register a raw file descriptor with the reactor for async I/O.
///
/// # Parameters
/// - `fd`: Raw file descriptor (as i64, cast to i32 on Unix).
/// - `interest_kind`: 1 = READABLE, 2 = WRITABLE, 3 = BOTH.
/// - `task_id`: The scheduler task ID (usize) to wake when ready.
/// - `worker_id`: The worker ID (usize) for waker affinity (currently unused).
///
/// # Returns
/// The mio Token value on success, or 0 on failure.
#[no_mangle]
pub extern "C" fn __reactor_register(
    fd: i64,
    interest_kind: i64,
    task_id: usize,
    worker_id: usize,
) -> i64 {
    let fd = fd as RawFd;
    let interest = match interest_kind {
        1 => Interest::READABLE,
        2 => Interest::WRITABLE,
        _ => Interest::READABLE.add(Interest::WRITABLE),
    };

    let waker = crate::async_runtime::make_waker(TaskId(task_id), worker_id);

    let reactor = match GLOBAL_REACTOR.lock() {
        Ok(r) => r,
        Err(_) => return 0,
    };

    match reactor.register(fd, interest, waker) {
        Ok(token) => token.0 as i64,
        Err(_) => 0,
    }
}

/// Re-register an existing reactor token with new interests.
///
/// # Returns 0 on success, -1 on failure.
#[no_mangle]
pub extern "C" fn __reactor_reregister(fd: i64, token: i64, interest_kind: i64) -> i64 {
    let fd = fd as RawFd;
    let token = Token(token as usize);
    let interest = match interest_kind {
        1 => Interest::READABLE,
        2 => Interest::WRITABLE,
        _ => Interest::READABLE.add(Interest::WRITABLE),
    };

    let reactor = match GLOBAL_REACTOR.lock() {
        Ok(r) => r,
        Err(_) => return -1,
    };

    match reactor.reregister(fd, token, interest) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Deregister a reactor token (e.g., when a socket is closed).
///
/// # Returns 0 on success, -1 on failure.
#[no_mangle]
pub extern "C" fn __reactor_deregister(fd: i64, token: i64) -> i64 {
    let fd = fd as RawFd;
    let token = Token(token as usize);

    let reactor = match GLOBAL_REACTOR.lock() {
        Ok(r) => r,
        Err(_) => return -1,
    };

    match reactor.deregister(fd, token) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reactor_creation() {
        let reactor = Reactor::new();
        assert!(reactor.is_ok());
    }

    #[test]
    fn test_reactor_poll_empty_timeout() {
        let reactor = Reactor::new().unwrap();
        let result = reactor.poll(Some(Duration::from_millis(0)));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_reactor_poll_no_timeout_quick() {
        let reactor = Reactor::new().unwrap();
        let result = reactor.poll(Some(Duration::from_millis(1)));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}
