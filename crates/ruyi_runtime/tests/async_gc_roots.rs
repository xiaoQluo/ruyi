use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ruyi_runtime::{
    alloc::TypeInfo,
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
