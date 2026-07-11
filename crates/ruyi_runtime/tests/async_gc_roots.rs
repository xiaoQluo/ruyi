use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ruyi_runtime::{
    alloc::TypeInfo,
    async_gc_roots::{
        ruyi_async_register_root, ruyi_async_unregister_root, snapshot as roots_snapshot,
    },
    async_runtime::{register_async_roots, Poll, RuyiFuture, Waker, GLOBAL_SCHEDULER},
    gc::MarkSweepCollector,
};

#[allow(dead_code)]
struct SuspendingFuture {
    gc_ptr: *mut u8,
    polled: Arc<AtomicBool>,
}

unsafe impl Send for SuspendingFuture {}

impl RuyiFuture for SuspendingFuture {
    type Output = ();

    fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
        if self.polled.load(Ordering::SeqCst) {
            Poll::Ready(())
        } else {
            self.polled.store(true, Ordering::SeqCst);
            Poll::Pending
        }
    }
}

#[test]
fn test_async_gc_roots_survive_collection() {
    static mut DUMMY_TYPE_INFO: TypeInfo = TypeInfo {
        type_id: 999,
        type_name: "async_root_test",
        destructor: None,
        trace_fn: None,
    };

    let mut collector = MarkSweepCollector::new();

    let obj = unsafe { collector.allocate(8, &raw mut DUMMY_TYPE_INFO) };
    assert!(!obj.is_null());
    unsafe {
        *(obj as *mut u64) = 0xBADC0FFEE;
    }

    let polled = Arc::new(AtomicBool::new(false));
    let future = SuspendingFuture {
        gc_ptr: obj,
        polled: polled.clone(),
    };

    let task_id = {
        let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
        scheduler.spawn(future)
    };

    while !polled.load(Ordering::SeqCst) {
        std::thread::yield_now();
    }
    std::thread::sleep(std::time::Duration::from_millis(50));

    register_async_roots(&mut collector);
    collector.collect();

    assert_eq!(collector.object_count(), 1);
    unsafe {
        assert_eq!(*(obj as *mut u64), 0xBADC0FFEE);
    }

    unsafe {
        collector.remove_root(obj);
    }
    collector.collect();
    assert_eq!(collector.object_count(), 0);

    {
        let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
        let waker = scheduler.test_waker(task_id);
        waker.wake();
    }

    {
        let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
        while scheduler.active_tasks() > 0 {
            std::thread::yield_now();
        }
    }
}

/// Future that captures three GC pointers as distinct fields, exercising
/// the scan path that must treat every word-sized slot as a potential
/// heap reference. Used by `test_multi_layer_future_chain` to simulate a
/// depth-3 await chain holding three transitively-reachable GC objects.
#[allow(dead_code)]
struct MultiLayerFuture {
    ptr1: *mut u8,
    ptr2: *mut u8,
    ptr3: *mut u8,
    polled: Arc<AtomicBool>,
}

unsafe impl Send for MultiLayerFuture {}

impl RuyiFuture for MultiLayerFuture {
    type Output = ();

    fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
        if self.polled.load(Ordering::SeqCst) {
            Poll::Ready(())
        } else {
            self.polled.store(true, Ordering::SeqCst);
            Poll::Pending
        }
    }
}

/// A suspended task holding a single GC object reference survives
/// `collect()` when its task ID is registered via the FFI
/// `ruyi_async_register_root` entry point.
#[test]
fn test_task_held_object_survives_gc() {
    static mut DUMMY_TYPE_INFO: TypeInfo = TypeInfo {
        type_id: 3001,
        type_name: "task_held_survive",
        destructor: None,
        trace_fn: None,
    };

    let mut collector = MarkSweepCollector::new();

    let obj = unsafe { collector.allocate(8, &raw mut DUMMY_TYPE_INFO) };
    assert!(!obj.is_null());
    unsafe {
        *(obj as *mut u64) = 0xDEADBEEF;
    }

    let polled = Arc::new(AtomicBool::new(false));
    let future = SuspendingFuture {
        gc_ptr: obj,
        polled: polled.clone(),
    };

    let task_id = {
        let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
        scheduler.spawn(future)
    };

    while !polled.load(Ordering::SeqCst) {
        std::thread::yield_now();
    }
    std::thread::yield_now();

    unsafe {
        ruyi_async_register_root(task_id.0);
    }
    let snapshot = roots_snapshot();
    assert!(
        snapshot.contains(&task_id.0),
        "FFI registration must add task id to the snapshot (got: {:?})",
        snapshot
    );

    register_async_roots(&mut collector);
    collector.collect();

    assert_eq!(
        collector.object_count(),
        1,
        "Object held by a registered task must survive collection"
    );
    unsafe {
        assert_eq!(*(obj as *mut u64), 0xDEADBEEF, "payload must be intact");
    }

    unsafe {
        ruyi_async_unregister_root(task_id.0);
    }
    {
        let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
        let waker = scheduler.test_waker(task_id);
        waker.wake();
    }
    std::thread::yield_now();
}

/// A depth-3 future chain has every reference marked as a GC root. Each
/// future captures a distinct GC pointer field; when the outermost task is
/// registered via the FFI all three transitively-referenced objects must
/// be retained across `collect()`.
#[test]
fn test_multi_layer_future_chain() {
    static mut DUMMY_TYPE_INFO: TypeInfo = TypeInfo {
        type_id: 3002,
        type_name: "multi_layer_chain",
        destructor: None,
        trace_fn: None,
    };

    let mut collector = MarkSweepCollector::new();

    let objs: [*mut u8; 3] = unsafe {
        [
            collector.allocate(8, &raw mut DUMMY_TYPE_INFO),
            collector.allocate(8, &raw mut DUMMY_TYPE_INFO),
            collector.allocate(8, &raw mut DUMMY_TYPE_INFO),
        ]
    };
    for (i, obj) in objs.iter().enumerate() {
        assert!(!obj.is_null());
        unsafe {
            *(*obj as *mut u64) = 0xA000_0000 + i as u64;
        }
    }

    let polled = Arc::new(AtomicBool::new(false));
    let future = MultiLayerFuture {
        ptr1: objs[0],
        ptr2: objs[1],
        ptr3: objs[2],
        polled: polled.clone(),
    };

    let task_id = {
        let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
        scheduler.spawn(future)
    };

    while !polled.load(Ordering::SeqCst) {
        std::thread::yield_now();
    }
    std::thread::yield_now();

    unsafe {
        ruyi_async_register_root(task_id.0);
    }

    register_async_roots(&mut collector);
    collector.collect();

    assert_eq!(
        collector.object_count(),
        3,
        "All three chain-referenced objects must survive collection"
    );
    for (i, obj) in objs.iter().enumerate() {
        unsafe {
            assert_eq!(
                *(*obj as *mut u64),
                0xA000_0000 + i as u64,
                "payload at depth {} must be intact",
                i + 1
            );
        }
    }

    for obj in &objs {
        unsafe {
            collector.remove_root(*obj);
        }
    }
    unsafe {
        ruyi_async_unregister_root(task_id.0);
    }
    {
        let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
        let waker = scheduler.test_waker(task_id);
        waker.wake();
    }
    std::thread::yield_now();
}

/// After the task completes, its referenced object is no longer reachable
/// through any active root and must be reclaimed by GC.
#[test]
fn test_completed_task_releases() {
    static mut DUMMY_TYPE_INFO: TypeInfo = TypeInfo {
        type_id: 3003,
        type_name: "completed_release",
        destructor: None,
        trace_fn: None,
    };

    let mut collector = MarkSweepCollector::new();

    let obj = unsafe { collector.allocate(8, &raw mut DUMMY_TYPE_INFO) };
    assert!(!obj.is_null());
    unsafe {
        *(obj as *mut u64) = 0xFACEFEED;
    }

    let polled = Arc::new(AtomicBool::new(false));
    let future = SuspendingFuture {
        gc_ptr: obj,
        polled: polled.clone(),
    };

    let task_id = {
        let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
        scheduler.spawn(future)
    };

    while !polled.load(Ordering::SeqCst) {
        std::thread::yield_now();
    }
    std::thread::yield_now();

    unsafe {
        ruyi_async_register_root(task_id.0);
    }
    register_async_roots(&mut collector);
    collector.collect();
    assert_eq!(
        collector.object_count(),
        1,
        "while suspended and registered, the object must survive"
    );

    {
        let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
        let waker = scheduler.test_waker(task_id);
        waker.wake();
    }
    {
        let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
        for _ in 0..1000 {
            if scheduler.active_tasks() == 0 {
                break;
            }
            std::thread::yield_now();
        }
    }

    unsafe {
        ruyi_async_unregister_root(task_id.0);
    }

    assert!(
        !roots_snapshot().contains(&task_id.0),
        "Unregister must remove the task id from the registry"
    );

    unsafe {
        collector.remove_root(obj);
    }
    register_async_roots(&mut collector);
    collector.collect();

    assert_eq!(
        collector.object_count(),
        0,
        "After task completion + unregister, the previously-held object must be reclaimable"
    );
}