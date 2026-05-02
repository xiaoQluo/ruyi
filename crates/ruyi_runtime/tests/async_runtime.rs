use ruyi_runtime::async_runtime::{
    ruyi_await, JoinAll, RuyiFuture, Poll, Race, Scheduler, TaskId, Waker,
    WorkStealingDeque,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct ImmediateFuture<T>(Option<T>);

impl<T> RuyiFuture for ImmediateFuture<T> {
    type Output = T;
    fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
        Poll::Ready(self.0.take().unwrap())
    }
}

struct YieldingFuture {
    yielded: bool,
}

impl RuyiFuture for YieldingFuture {
    type Output = i32;
    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        if self.yielded {
            Poll::Ready(42)
        } else {
            self.yielded = true;
            waker.wake();
            Poll::Pending
        }
    }
}

#[test]
fn test_scheduler_spawn_and_run() {
    let scheduler = Scheduler::new(2);
    let completed = Arc::new(AtomicUsize::new(0));
    let c1 = completed.clone();

    struct CountingFuture {
        counter: Arc<AtomicUsize>,
    }
    impl RuyiFuture for CountingFuture {
        type Output = ();
        fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            Poll::Ready(())
        }
    }

    scheduler.spawn(CountingFuture { counter: c1 });
    scheduler.block_on_all();
    assert_eq!(completed.load(Ordering::SeqCst), 1);
    scheduler.shutdown();
}

#[test]
fn test_work_stealing_deque() {
    let deque = WorkStealingDeque::new();
    deque.push_bottom(1);
    deque.push_bottom(2);
    deque.push_bottom(3);
    assert_eq!(deque.pop_bottom(), Some(3));
    assert_eq!(deque.steal_top(), Some(1));
    assert_eq!(deque.pop_bottom(), Some(2));
    assert_eq!(deque.pop_bottom(), None);
}

#[test]
fn test_join_all() {
    let futs = vec![
        ImmediateFuture(Some(1)),
        ImmediateFuture(Some(2)),
        ImmediateFuture(Some(3)),
    ];
    let mut join = JoinAll::new(futs);
    let scheduler = Scheduler::new(1);
    let waker = scheduler.test_waker(TaskId(0));
    assert_eq!(join.poll(&waker), Poll::Ready(vec![1, 2, 3]));
    scheduler.shutdown();
}

#[test]
fn test_race() {
    let futs = vec![
        ImmediateFuture(Some(1)),
        ImmediateFuture(Some(2)),
    ];
    let mut race = Race::new(futs);
    let scheduler = Scheduler::new(1);
    let waker = scheduler.test_waker(TaskId(0));
    assert_eq!(race.poll(&waker), Poll::Ready(1));
    scheduler.shutdown();
}

#[test]
fn test_yielding_future() {
    let mut fut = YieldingFuture { yielded: false };
    let scheduler = Scheduler::new(1);
    let waker = scheduler.test_waker(TaskId(0));
    assert_eq!(fut.poll(&waker), Poll::Pending);
    assert_eq!(fut.poll(&waker), Poll::Ready(42));
    scheduler.shutdown();
}

#[test]
fn test_ruyi_await() {
    let ptr: *mut u8 = 0x1234 as *mut u8;
    let result = ruyi_await(ptr);
    assert_eq!(result, ptr);
}

#[test]
fn test_scheduler_multiple_tasks() {
    struct IncrementFuture(Arc<AtomicUsize>);
    impl RuyiFuture for IncrementFuture {
        type Output = ();
        fn poll(&mut self, _waker: &Waker) -> Poll<Self::Output> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Poll::Ready(())
        }
    }

    let scheduler = Scheduler::new(4);
    let counter = Arc::new(AtomicUsize::new(0));

    for _ in 0..10 {
        let c = counter.clone();
        scheduler.spawn(IncrementFuture(c));
    }

    scheduler.block_on_all();
    assert_eq!(counter.load(Ordering::SeqCst), 10);
    scheduler.shutdown();
}

#[test]
fn test_task_id_unique() {
    let scheduler = Scheduler::new(1);
    let id1 = scheduler.spawn(ImmediateFuture(Some(())));
    let id2 = scheduler.spawn(ImmediateFuture(Some(())));
    assert_ne!(id1, id2);
    scheduler.shutdown();
}
