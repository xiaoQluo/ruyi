use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ruyi_runtime::{
    alloc::TypeInfo,
    async_runtime::{register_async_roots, Poll, RuyiFuture, Waker, GLOBAL_SCHEDULER},
    gc::MarkSweepCollector,
};

struct SuspendingFuture {
    // Never read by Rust code: kept in the future's memory so the
    // word-wise async-root scan can discover the GC pointer.
    #[allow(dead_code)]
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

struct CompletedFuture {
    marker: Arc<AtomicBool>,
}

unsafe impl Send for CompletedFuture {}

impl RuyiFuture for CompletedFuture {
    type Output = ();

    fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
        self.marker.store(true, Ordering::SeqCst);
        Poll::Ready(())
    }
}

#[test]
fn test_register_async_roots_scans_task_objects() {
    static mut DUMMY_TYPE_INFO: TypeInfo = TypeInfo {
        type_id: 1001,
        type_name: "async_root_scan_test",
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

    wake_and_join_task(task_id);
}

#[test]
fn test_suspended_task_references_survive_collection() {
    static mut DUMMY_TYPE_INFO: TypeInfo = TypeInfo {
        type_id: 1002,
        type_name: "suspended_ref_test",
        destructor: None,
        trace_fn: None,
    };

    let mut collector = MarkSweepCollector::new();

    let obj = unsafe { collector.allocate(8, &raw mut DUMMY_TYPE_INFO) };
    assert!(!obj.is_null());
    unsafe {
        *(obj as *mut u64) = 0xCAFEBABE;
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
        assert_eq!(*(obj as *mut u64), 0xCAFEBABE);
    }

    unsafe {
        collector.remove_root(obj);
    }
    collector.collect();
    assert_eq!(collector.object_count(), 0);

    wake_and_join_task(task_id);
}

#[test]
fn test_multiple_tasks_each_have_independent_roots() {
    static mut DUMMY_TYPE_INFO: TypeInfo = TypeInfo {
        type_id: 1003,
        type_name: "multi_task_roots",
        destructor: None,
        trace_fn: None,
    };

    let mut collector = MarkSweepCollector::new();

    let objs: Vec<*mut u8> = (0..3)
        .map(|_i| unsafe { collector.allocate(8, &raw mut DUMMY_TYPE_INFO) })
        .collect();

    for (i, obj) in objs.iter().enumerate() {
        assert!(!obj.is_null());
        unsafe {
            *(*obj as *mut u64) = (0x1000 + i) as u64;
        }
    }

    struct LocalSuspendingFuture {
        // Same as SuspendingFuture: present only for the root scan.
        #[allow(dead_code)]
        gc_ptr: *mut u8,
        polled: Arc<AtomicBool>,
    }

    unsafe impl Send for LocalSuspendingFuture {}

    impl RuyiFuture for LocalSuspendingFuture {
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

    let polled1 = Arc::new(AtomicBool::new(false));
    let polled2 = Arc::new(AtomicBool::new(false));
    let polled3 = Arc::new(AtomicBool::new(false));

    let task_ids: Vec<_> = {
        let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
        vec![
            scheduler.spawn(LocalSuspendingFuture {
                gc_ptr: objs[0],
                polled: polled1.clone(),
            }),
            scheduler.spawn(LocalSuspendingFuture {
                gc_ptr: objs[1],
                polled: polled2.clone(),
            }),
            scheduler.spawn(LocalSuspendingFuture {
                gc_ptr: objs[2],
                polled: polled3.clone(),
            }),
        ]
    };

    while !polled1.load(Ordering::SeqCst)
        || !polled2.load(Ordering::SeqCst)
        || !polled3.load(Ordering::SeqCst)
    {
        std::thread::yield_now();
    }
    std::thread::sleep(std::time::Duration::from_millis(50));

    register_async_roots(&mut collector);
    collector.collect();

    assert_eq!(collector.object_count(), 3);
    for (i, obj) in objs.iter().enumerate() {
        unsafe {
            assert_eq!(*(*obj as *mut u64), (0x1000 + i) as u64);
        }
    }

    for obj in &objs {
        unsafe {
            collector.remove_root(*obj);
        }
    }
    collector.collect();
    assert_eq!(collector.object_count(), 0);

    // Wake every task first: joining after each wake would wait for the
    // global active count to reach zero while the remaining tasks are
    // still suspended, which never terminates.
    for task_id in &task_ids {
        wake_task(*task_id);
    }
    join_all_tasks();
}

#[test]
fn test_completed_task_objects_can_be_collected() {
    static mut DUMMY_TYPE_INFO: TypeInfo = TypeInfo {
        type_id: 1004,
        type_name: "completed_task_test",
        destructor: None,
        trace_fn: None,
    };

    let collector = MarkSweepCollector::new();

    let obj = unsafe { collector.allocate(8, &raw mut DUMMY_TYPE_INFO) };
    assert!(!obj.is_null());
    unsafe {
        *(obj as *mut u64) = 0xFEEDFACE;
    }

    let completed = Arc::new(AtomicBool::new(false));
    let task_id = {
        let future = CompletedFuture {
            marker: completed.clone(),
        };
        let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
        scheduler.spawn(future)
    };

    while !completed.load(Ordering::SeqCst) {
        std::thread::yield_now();
    }
    std::thread::sleep(std::time::Duration::from_millis(50));

    collector.collect();
    assert_eq!(collector.object_count(), 0);

    wake_and_join_task(task_id);
}

#[test]
fn test_task_completion_releases_gc_roots() {
    static mut DUMMY_TYPE_INFO: TypeInfo = TypeInfo {
        type_id: 1005,
        type_name: "root_release_test",
        destructor: None,
        trace_fn: None,
    };

    let mut collector = MarkSweepCollector::new();

    let obj = unsafe { collector.allocate(8, &raw mut DUMMY_TYPE_INFO) };
    assert!(!obj.is_null());
    unsafe {
        *(obj as *mut u64) = 0x5EC00000;
    }

    let polled = Arc::new(AtomicBool::new(false));
    let task_id = {
        let future = SuspendingFuture {
            gc_ptr: obj,
            polled: polled.clone(),
        };
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
        assert_eq!(*(obj as *mut u64), 0x5EC00000);
    }

    {
        let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
        let waker = scheduler.test_waker(task_id);
        waker.wake();
    }

    // Poll without holding the scheduler lock: spinning while holding it
    // deadlocks parallel tests that need the lock to wake their own tasks.
    loop {
        let active = GLOBAL_SCHEDULER.lock().unwrap().active_tasks();
        if active == 0 {
            break;
        }
        std::thread::yield_now();
    }
    std::thread::sleep(std::time::Duration::from_millis(50));

    unsafe {
        collector.remove_root(obj);
    }
    collector.collect();
    assert_eq!(
        collector.object_count(),
        0,
        "Completed task's referenced objects should be collectible"
    );
}

fn wake_task(task_id: ruyi_runtime::async_runtime::TaskId) {
    let scheduler = GLOBAL_SCHEDULER.lock().unwrap();
    let waker = scheduler.test_waker(task_id);
    waker.wake();
}

fn join_all_tasks() {
    // Re-acquire the lock on each iteration; holding it across the spin
    // starves the worker thread and sibling tests.
    loop {
        let active = GLOBAL_SCHEDULER.lock().unwrap().active_tasks();
        if active == 0 {
            break;
        }
        std::thread::yield_now();
    }
}

fn wake_and_join_task(task_id: ruyi_runtime::async_runtime::TaskId) {
    wake_task(task_id);
    join_all_tasks();
}
