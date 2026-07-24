/**
 * Integration tests for thread/channel/rwlock FFI modules.
 *
 * Exercises the full coordination path: spawn threads → communicate
 * via channels → protect shared state with RWLock.
 *
 * Since thread entry points are `extern "C" fn(usize)`, multi-argument
 * payloads are packed into heap-allocated structs passed via `usize`.
 *
 * @author Ruyi Team
 * @date 2026-07-25
 */
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ruyi_runtime::channel_ffi::*;
use ruyi_runtime::rwlock_ffi::*;
use ruyi_runtime::thread_ffi::*;

// ── Payload structs for multi-arg entries ───────────────────────

struct ProducerCtx {
    ch: *mut i8,
    start: i64,
    count: i64,
}

struct WriterCtx {
    rw: *mut i8,
    counter: *const AtomicI64,
    iterations: usize,
}

struct WorkerCtx {
    ch: *mut i8,
    rw: *mut i8,
    id: i64,
}

// ── Thread entry functions (all `extern "C" fn(usize)`) ─────────

extern "C" fn entry_counter(arg: usize) {
    let counter = unsafe { &*(arg as *const AtomicI64) };
    counter.fetch_add(1, Ordering::SeqCst);
}

extern "C" fn entry_producer(arg: usize) {
    let ch = arg as *mut i8;
    for i in 0..10 {
        __channel_send(ch, i);
    }
}

extern "C" fn entry_reader(arg: usize) {
    let rw = arg as *mut i8;
    let guard = __rwlock_read_lock(rw);
    assert!(!guard.is_null());
    std::thread::sleep(Duration::from_millis(10));
    __rwlock_read_unlock(guard);
}

extern "C" fn entry_mp_producer(arg: usize) {
    let ctx = unsafe { Box::from_raw(arg as *mut ProducerCtx) };
    for i in 0..ctx.count {
        __channel_send(ctx.ch, ctx.start + i);
    }
    // ctx is dropped — avoid double-free by not calling from_raw again
    std::mem::forget(ctx);
}

extern "C" fn entry_writer(arg: usize) {
    let ctx = unsafe { Box::from_raw(arg as *mut WriterCtx) };
    for _ in 0..ctx.iterations {
        let guard = __rwlock_write_lock(ctx.rw);
        let counter = unsafe { &*ctx.counter };
        counter.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(1));
        __rwlock_write_unlock(guard);
    }
    std::mem::forget(ctx);
}

extern "C" fn entry_worker(arg: usize) {
    let ctx = unsafe { Box::from_raw(arg as *mut WorkerCtx) };
    // Read lock on shared state
    let g = __rwlock_read_lock(ctx.rw);
    assert!(!g.is_null());
    __rwlock_read_unlock(g);
    // Send result
    __channel_send(ctx.ch, ctx.id * 100);
    std::mem::forget(ctx);
}

// ── Tests ───────────────────────────────────────────────────────

#[test]
fn test_integration_spawn_multiple_threads() {
    let counter = Arc::new(AtomicI64::new(0));
    let ptr = Arc::as_ptr(&counter) as usize;

    let mut handles = Vec::new();
    for _ in 0..5 {
        let h = __thread_spawn(entry_counter as *mut i8, ptr as *mut i8);
        assert!(h > 0);
        handles.push(h);
    }
    for h in handles {
        assert_eq!(__thread_join(h), 0);
    }
    assert_eq!(counter.load(Ordering::SeqCst), 5);
}

#[test]
fn test_integration_channel_producer_consumer() {
    let ch = __channel_new(0);
    assert!(!ch.is_null());

    // Producer in thread, consumer in main
    let handle = __thread_spawn(entry_producer as *mut i8, ch as *mut i8);
    assert!(handle > 0);

    let mut sum: i64 = 0;
    for _ in 0..10 {
        sum += __channel_recv(ch);
    }
    __thread_join(handle);
    __channel_free(ch);

    assert_eq!(sum, 45); // 0+1+...+9
}

#[test]
fn test_integration_rwlock_concurrent_readers() {
    let rw = __rwlock_new();
    let rw_ptr = rw as usize;

    let mut handles = Vec::new();
    for _ in 0..3 {
        let h = __thread_spawn(entry_reader as *mut i8, rw_ptr as *mut i8);
        assert!(h > 0);
        handles.push(h);
    }

    // Main thread also acquires read lock
    let guard = __rwlock_read_lock(rw);
    assert!(!guard.is_null());

    for h in handles {
        __thread_join(h);
    }
    __rwlock_read_unlock(guard);
    __rwlock_free(rw);
}

#[test]
fn test_integration_rwlock_writers_serial() {
    let rw = __rwlock_new();
    let counter = Arc::new(AtomicI64::new(0));

    let ctx = Box::new(WriterCtx {
        rw,
        counter: Arc::as_ptr(&counter),
        iterations: 3,
    });
    let arg = Box::into_raw(ctx) as usize;

    let h1 = __thread_spawn(entry_writer as *mut i8, arg as *mut i8);
    let h2 = __thread_spawn(entry_writer as *mut i8, arg as *mut i8);
    assert!(h1 > 0 && h2 > 0);

    __thread_join(h1);
    __thread_join(h2);
    __rwlock_free(rw);

    // 2 writers × 3 iterations = 6 writes
    assert_eq!(counter.load(Ordering::SeqCst), 6);
}

#[test]
fn test_integration_channel_multi_producer() {
    let ch = __channel_new(0);

    let ctx = Box::new(ProducerCtx {
        ch,
        start: 0,
        count: 5,
    });
    let h1_arg = Box::into_raw(ctx) as usize;
    let ctx = Box::new(ProducerCtx {
        ch,
        start: 5,
        count: 5,
    });
    let h2_arg = Box::into_raw(ctx) as usize;
    let ctx = Box::new(ProducerCtx {
        ch,
        start: 10,
        count: 5,
    });
    let h3_arg = Box::into_raw(ctx) as usize;

    let h1 = __thread_spawn(entry_mp_producer as *mut i8, h1_arg as *mut i8);
    let h2 = __thread_spawn(entry_mp_producer as *mut i8, h2_arg as *mut i8);
    let h3 = __thread_spawn(entry_mp_producer as *mut i8, h3_arg as *mut i8);

    let mut received = Vec::new();
    for _ in 0..15 {
        received.push(__channel_recv(ch));
    }
    __thread_join(h1);
    __thread_join(h2);
    __thread_join(h3);
    __channel_free(ch);

    received.sort();
    assert_eq!(received, (0..15).collect::<Vec<i64>>());
}

#[test]
fn test_integration_full_coordination() {
    let ch = __channel_new(0);
    let rw = __rwlock_new();

    let mut handles = Vec::new();
    for id in 0..4 {
        let ctx = Box::new(WorkerCtx { ch, rw, id });
        let arg = Box::into_raw(ctx) as usize;
        let h = __thread_spawn(entry_worker as *mut i8, arg as *mut i8);
        assert!(h > 0);
        handles.push(h);
    }

    let mut results = Vec::new();
    for _ in 0..4 {
        results.push(__channel_recv(ch));
    }
    for h in handles {
        __thread_join(h);
    }
    __channel_free(ch);
    __rwlock_free(rw);

    results.sort();
    assert_eq!(results, vec![0, 100, 200, 300]);
}
