//! Async runtime: green thread scheduler, task queue, and executor.
//!
//! Implements the Ruyi async/await runtime per spec Section 14:
//! - Work-stealing scheduler with per-worker task queues
//! - Future state machine abstraction (`Poll<T>`, `Task`)
//! - Waker mechanism for I/O and timer-driven wakeups
//! - GC integration: async tasks are traced as roots
//!
//! @author Ruyi Team
//! @date 2026-05-01

use once_cell::sync::Lazy;
use std::cell::Cell;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};

extern "C" {
    fn ruyi_async_register_root(task_id: usize);
    fn ruyi_async_unregister_root(task_id: usize);
}

thread_local! {
    static IS_WORKER_THREAD: Cell<bool> = const { Cell::new(false) };
}

fn is_worker_thread() -> bool {
    IS_WORKER_THREAD.with(|f| f.get())
}

// ── Poll / Future core types ─────────────────────────────────

/// Result of polling a future.
#[derive(Debug, Clone, PartialEq)]
pub enum Poll<T> {
    /// The future is complete with the given value.
    Ready(T),
    /// The future is not yet complete; it will be woken later.
    Pending,
}

/// A Ruyi future that can be polled.
pub trait RuyiFuture {
    /// The type of value produced on completion.
    type Output;

    /// Attempt to make progress.
    ///
    /// If the future is not ready, it must arrange for `wake()` to be
    /// called on the provided `Waker` when it can make further progress.
    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output>;
}

// ── Waker ────────────────────────────────────────────────────

/// A handle used to re-schedule a task when it can make progress.
#[derive(Clone)]
pub struct Waker {
    /// Shared scheduler reference used to enqueue the task.
    pub(crate) scheduler: Arc<SchedulerInner>,
    /// Index of the worker that originally polled this task.
    pub(crate) worker_id: usize,
    /// Task id for re-queueing.
    pub(crate) task_id: TaskId,
}

impl Waker {
    /// Signal that the associated task should be re-polled.
    pub fn wake(&self) {
        self.scheduler.wake_task(self.task_id, self.worker_id);
    }
}

// ── Task ─────────────────────────────────────────────────────

/// Unique identifier for a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub usize);

/// Internal representation of a runnable task.
pub struct Task {
    pub id: TaskId,
    /// Boxed future so tasks of different types can coexist.
    pub future: Box<dyn RuyiFuture<Output = ()> + Send>,
    /// Whether the task has been woken since last poll.
    pub woken: bool,
}

impl Task {
    pub fn new(id: TaskId, future: Box<dyn RuyiFuture<Output = ()> + Send>) -> Self {
        Self {
            id,
            future,
            woken: true,
        }
    }
}

// ── Work-stealing deque ──────────────────────────────────────

/// A simple work-stealing deque used by each scheduler worker.
pub struct WorkStealingDeque<T> {
    inner: Mutex<VecDeque<T>>,
}

impl<T> WorkStealingDeque<T> {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(VecDeque::new()),
        }
    }

    /// Push to the bottom (local end).
    pub fn push_bottom(&self, item: T) {
        self.inner.lock().unwrap().push_back(item);
    }

    /// Pop from the bottom (local end).
    pub fn pop_bottom(&self) -> Option<T> {
        self.inner.lock().unwrap().pop_back()
    }

    /// Steal from the top (remote end).
    pub fn steal_top(&self) -> Option<T> {
        self.inner.lock().unwrap().pop_front()
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T> Default for WorkStealingDeque<T> {
    fn default() -> Self {
        Self::new()
    }
}

// ── Scheduler ────────────────────────────────────────────────

/// Shared scheduler state.
pub struct SchedulerInner {
    /// Per-worker local queues.
    workers: Vec<Worker>,
    /// Global run queue for tasks that don't have an affinity.
    global_queue: Mutex<VecDeque<TaskId>>,
    /// Mapping from task id to the actual task.
    pub(crate) tasks: Mutex<std::collections::HashMap<TaskId, Box<Task>>>,
    /// Wakes that arrived while the task was checked out by a worker
    /// (removed from `tasks` for the duration of a poll). Applied when the
    /// task is re-inserted so the wakeup is not lost, which would otherwise
    /// strand the task forever (its queued id pops with `woken == false`).
    inflight_wakes: Mutex<std::collections::HashSet<TaskId>>,
    /// Next task id.
    next_task_id: AtomicUsize,
    /// Condition variable used to park idle workers.
    park_cond: Condvar,
    /// Number of active tasks (not yet completed).
    active_count: AtomicUsize,
    /// Set to true when the scheduler should shut down.
    shutdown: AtomicUsize,
}

struct Worker {
    queue: WorkStealingDeque<TaskId>,
}

impl SchedulerInner {
    pub(crate) fn new(num_workers: usize) -> Arc<Self> {
        let workers: Vec<Worker> = (0..num_workers)
            .map(|_| Worker {
                queue: WorkStealingDeque::new(),
            })
            .collect();

        Arc::new(Self {
            workers,
            global_queue: Mutex::new(VecDeque::new()),
            tasks: Mutex::new(std::collections::HashMap::new()),
            inflight_wakes: Mutex::new(std::collections::HashSet::new()),
            next_task_id: AtomicUsize::new(1),
            park_cond: Condvar::new(),
            active_count: AtomicUsize::new(0),
            shutdown: AtomicUsize::new(0),
        })
    }

    /// Allocate a new task id.
    fn next_id(&self) -> TaskId {
        TaskId(self.next_task_id.fetch_add(1, Ordering::SeqCst))
    }

    /// Spawn a new task on the given worker (or round-robin if `preferred_worker` is out of range).
    fn spawn_task(
        self: &Arc<Self>,
        future: Box<dyn RuyiFuture<Output = ()> + Send>,
        preferred_worker: usize,
    ) -> TaskId {
        let id = self.next_id();
        let task = Box::new(Task::new(id, future));
        self.tasks.lock().unwrap().insert(id, task);
        self.active_count.fetch_add(1, Ordering::SeqCst);

        let worker_id = preferred_worker % self.workers.len();
        self.workers[worker_id].queue.push_bottom(id);

        // Wake up a potentially parked worker.
        self.park_cond.notify_one();

        id
    }

    /// Re-queue a task that was woken.
    fn wake_task(self: &Arc<Self>, task_id: TaskId, _preferred_worker: usize) {
        {
            let mut tasks = self.tasks.lock().unwrap();
            if let Some(task) = tasks.get_mut(&task_id) {
                task.woken = true;
            } else {
                // Task is checked out by a worker mid-poll (or already
                // completed). Record the wake so a `Poll::Pending`
                // re-insert picks it up instead of dropping it.
                self.inflight_wakes.lock().unwrap().insert(task_id);
            }
        }
        // Push to global queue to avoid lock contention on the worker deque.
        self.global_queue.lock().unwrap().push_back(task_id);
        self.park_cond.notify_one();
    }

    /// Attempt to find the next runnable task for the given worker.
    fn next_task(self: &Arc<Self>, worker_id: usize) -> Option<TaskId> {
        // 1. Try local queue.
        if let Some(id) = self.workers[worker_id].queue.pop_bottom() {
            return Some(id);
        }

        // 2. Try global queue.
        if let Some(id) = self.global_queue.lock().unwrap().pop_front() {
            return Some(id);
        }

        // 3. Try stealing from another worker.
        for i in 0..self.workers.len() {
            let victim = (worker_id + i + 1) % self.workers.len();
            if let Some(id) = self.workers[victim].queue.steal_top() {
                return Some(id);
            }
        }

        None
    }

    /// Mark a task as completed and remove it.
    fn complete_task(self: &Arc<Self>, id: TaskId) {
        self.tasks.lock().unwrap().remove(&id);
        // Drop any wake recorded while the final poll was in flight.
        self.inflight_wakes.lock().unwrap().remove(&id);
        self.active_count.fetch_sub(1, Ordering::SeqCst);
        self.park_cond.notify_all();
    }

    fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst) != 0
    }
}

/// Create a Waker for use by external systems (e.g., the I/O reactor).
///
/// This is the public constructor for `Waker`, allowing other runtime modules
/// to create wakers that can re-schedule tasks via the global scheduler.
///
/// Built from the shared scheduler core (`GLOBAL_INNER`) without taking the
/// outer `GLOBAL_SCHEDULER` lock: callers may run on worker threads while
/// another thread holds that lock, which would otherwise deadlock.
pub fn make_waker(task_id: TaskId, worker_id: usize) -> Waker {
    Lazy::force(&GLOBAL_SCHEDULER);
    Waker {
        scheduler: GLOBAL_INNER.clone(),
        worker_id,
        task_id,
    }
}

/// High-level green-thread scheduler.
pub struct Scheduler {
    pub(crate) inner: Arc<SchedulerInner>,
    /// OS thread handles for the worker threads.
    threads: Vec<std::thread::JoinHandle<()>>,
}

impl Scheduler {
    /// Create a new scheduler with `num_workers` OS threads.
    pub fn new(num_workers: usize) -> Self {
        Self::from_shared(SchedulerInner::new(num_workers))
    }

    /// Build a scheduler around an existing shared core, spawning one OS
    /// worker thread per core worker slot.
    fn from_shared(inner: Arc<SchedulerInner>) -> Self {
        let num_workers = inner.workers.len();
        let mut threads = Vec::with_capacity(num_workers);

        for worker_id in 0..num_workers {
            let scheduler = inner.clone();
            let handle = std::thread::spawn(move || worker_loop(scheduler, worker_id));
            threads.push(handle);
        }

        Self { inner, threads }
    }

    /// Spawn a future onto the scheduler.
    pub fn spawn<F>(&self, future: F) -> TaskId
    where
        F: RuyiFuture<Output = ()> + Send + 'static,
    {
        self.inner.spawn_task(Box::new(future), 0)
    }

    /// Block the current thread until all tasks have completed.
    pub fn block_on_all(&self) {
        while self.inner.active_count.load(Ordering::SeqCst) > 0 && !self.inner.is_shutdown() {
            std::thread::yield_now();
        }
    }

    pub fn suspend_current(&self, _task_id: TaskId) {
        self.block_on_all();
    }

    /// Shut down the scheduler and wait for workers to finish.
    pub fn shutdown(self) {
        self.inner.shutdown.store(1, Ordering::SeqCst);
        self.inner.park_cond.notify_all();
        for t in self.threads {
            let _ = t.join();
        }
    }

    /// Return the number of tasks currently active.
    pub fn active_tasks(&self) -> usize {
        self.inner.active_count.load(Ordering::SeqCst)
    }

    /// Create a dummy waker for testing.
    pub fn test_waker(&self, task_id: TaskId) -> Waker {
        Waker {
            scheduler: self.inner.clone(),
            worker_id: 0,
            task_id,
        }
    }
}

/// Worker thread main loop.
fn worker_loop(scheduler: Arc<SchedulerInner>, worker_id: usize) {
    IS_WORKER_THREAD.set(true);
    loop {
        if scheduler.is_shutdown() {
            break;
        }

        if let Some(task_id) = scheduler.next_task(worker_id) {
            let task_opt = scheduler.tasks.lock().unwrap().remove(&task_id);
            if let Some(mut task) = task_opt {
                if task.woken {
                    task.woken = false;
                    let waker = Waker {
                        scheduler: scheduler.clone(),
                        worker_id,
                        task_id,
                    };
                    match task.future.poll(&waker) {
                        Poll::Ready(_) => {
                            // Drop GC-root registration: future is going away.
                            unsafe { ruyi_async_unregister_root(task_id.0) };
                            scheduler.complete_task(task_id);
                        }
                        Poll::Pending => {
                            // Register GC root so parked future's heap refs survive.
                            unsafe { ruyi_async_register_root(task_id.0) };
                            // Apply any wake that raced with this poll while the
                            // task was out of the map (its id is already queued);
                            // hold the tasks lock across check + insert so no
                            // wake can slip between them.
                            let mut tasks = scheduler.tasks.lock().unwrap();
                            if scheduler.inflight_wakes.lock().unwrap().remove(&task_id) {
                                task.woken = true;
                            }
                            tasks.insert(task_id, task);
                        }
                    }
                } else {
                    // Task wasn't woken — put it back on a queue for later.
                    scheduler.tasks.lock().unwrap().insert(task_id, task);
                }
            }
        } else {
            // No tasks to run — drain any ready I/O events without blocking,
            // then park on the condvar. Blocking inside `reactor.poll` here
            // was uninterruptible by `wake_task`/`spawn_task` notifications
            // and stalled task completion for up to the full poll timeout.
            let woke = crate::reactor::GLOBAL_REACTOR
                .lock()
                .ok()
                .and_then(|reactor| reactor.poll(Some(std::time::Duration::ZERO)).ok())
                .unwrap_or(0);
            if woke == 0 {
                // Re-check the queue under the lock before parking: a wake
                // that lands between `next_task` and this point would fire
                // `notify_one` before the wait starts and be lost, leaving
                // the task to sit out the full park timeout.
                let guard = scheduler.global_queue.lock().unwrap();
                if guard.is_empty() {
                    let _ = scheduler
                        .park_cond
                        .wait_timeout(guard, std::time::Duration::from_millis(10))
                        .unwrap();
                }
            }
        }
    }
}

struct RawFuture {
    ptr: *mut u8,
}

unsafe impl Send for RawFuture {}

impl RuyiFuture for RawFuture {
    type Output = ();

    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        type PollFn = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
        let poll_fn_ptr = unsafe {
            let ptr_val = std::ptr::read::<*mut u8>(self.ptr as *const *mut u8);
            std::mem::transmute::<*mut u8, PollFn>(ptr_val)
        };
        let waker_ptr = waker as *const Waker as *mut u8;
        let result = unsafe { poll_fn_ptr(self.ptr, waker_ptr) };
        if result == 1 {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

// ── GC integration ───────────────────────────────────────────

#[no_mangle]
pub extern "C" fn ruyi_await(future: *mut u8) -> *mut u8 {
    if future.is_null() {
        return std::ptr::null_mut();
    }

    if is_worker_thread() {
        return future;
    }

    let _task_id = GLOBAL_SCHEDULER
        .lock()
        .unwrap()
        .spawn(RawFuture { ptr: future });
    // Wait for completion without holding the outer scheduler lock so
    // that worker threads and wakers can still reach the scheduler.
    while GLOBAL_INNER.active_count.load(Ordering::SeqCst) > 0 && !GLOBAL_INNER.is_shutdown() {
        std::thread::yield_now();
    }
    future
}

/// Exception state that can be stored inside a future.
///
/// When an async function throws, the exception is captured here
/// instead of unwinding the stack, so that the awaiting context
/// can rethrow it.
pub struct AsyncException {
    pub exception_ptr: *mut crate::exception::types::ExceptionObject,
}

impl AsyncException {
    pub fn new(ptr: *mut crate::exception::types::ExceptionObject) -> Self {
        Self { exception_ptr: ptr }
    }

    pub unsafe fn rethrow(&self) -> ! {
        crate::exception::runtime::ruyi_throw(self.exception_ptr)
    }
}

/// Shared core of the global scheduler.
///
/// Accessible without taking the outer `GLOBAL_SCHEDULER` mutex: GC root
/// scans and waker construction must work even while another thread holds
/// that lock, so they read the core directly through this handle.
pub(crate) static GLOBAL_INNER: Lazy<Arc<SchedulerInner>> = Lazy::new(|| SchedulerInner::new(1));

/// Global scheduler instance (baseline: single worker thread).
///
/// Uses the same `Lazy<Mutex<…>>` pattern as `gc_exports.rs`. Built on top
/// of `GLOBAL_INNER` so both handles refer to the same scheduler core.
pub static GLOBAL_SCHEDULER: Lazy<Mutex<Scheduler>> =
    Lazy::new(|| Mutex::new(Scheduler::from_shared(GLOBAL_INNER.clone())));

/// Register all active async tasks as GC roots.
///
/// This should be called before each GC collection cycle so that objects
/// reachable from suspended async functions are not collected.
///
/// When the FFI registry (`async_gc_roots`) holds at least one task id
/// the scan is filtered to only those ids — this matches the suspended-
/// task semantics enforced by the FFI entry points and prevents the
/// collector from pinning references held by long-running ready tasks.
/// With an empty registry the scan falls back to the previous
/// "every task is a root" behaviour so callers that pre-date the FFI
/// keep working.
pub fn register_async_roots(collector: &mut crate::gc::MarkSweepCollector) {
    let filter: Option<HashSet<usize>> = {
        let snap = crate::async_gc_roots::snapshot();
        if snap.is_empty() {
            None
        } else {
            Some(snap.into_iter().collect())
        }
    };

    // The worker removes a task from `tasks` for the duration of a poll.
    // If a GC cycle coincides with that window, the task's future is
    // invisible to the scan and its referenced objects would be
    // incorrectly reclaimed.  When filtering by registry, briefly wait
    // for every registered task to be back in the map before scanning.
    if let Some(ref ids) = filter {
        for _ in 0..500 {
            let tasks = GLOBAL_INNER.tasks.lock().unwrap();
            let all_present = ids.iter().all(|id| tasks.contains_key(&TaskId(*id)));
            drop(tasks);
            if all_present {
                break;
            }
            std::thread::yield_now();
        }
    }

    // Scan through the shared core directly: taking (or try-taking) the
    // outer `GLOBAL_SCHEDULER` lock here silently skipped the scan when
    // the lock was contended, letting live async-held objects be swept.
    let tasks = GLOBAL_INNER.tasks.lock().unwrap();
    for (task_id, task) in tasks.iter() {
        if let Some(ref allowed) = filter {
            if !allowed.contains(&task_id.0) {
                continue;
            }
        }
        let future_ref: &(dyn RuyiFuture<Output = ()> + Send) = &*task.future;
        let data_ptr = future_ref as *const (dyn RuyiFuture<Output = ()> + Send) as *const u8;
        let size = std::mem::size_of_val(future_ref);
        let step = std::mem::size_of::<usize>();
        if data_ptr.is_null() || size == 0 {
            continue;
        }
        let mut offset = 0;
        while offset + step <= size {
            let word = unsafe { std::ptr::read_unaligned(data_ptr.add(offset) as *const usize) };
            let candidate = word as *mut u8;
            if !candidate.is_null() && collector.is_valid_payload(candidate) {
                unsafe {
                    collector.add_root(candidate);
                }
            }
            offset += step;
        }
    }
}

// ── Combinators ──────────────────────────────────────────────

/// Wait for all futures to complete and return their results.
pub struct JoinAll<F, T> {
    futures: Vec<(usize, F)>,
    results: Vec<Option<T>>,
    completed: usize,
}

impl<F, T> JoinAll<F, T>
where
    F: RuyiFuture<Output = T>,
{
    pub fn new(futures: Vec<F>) -> Self {
        let n = futures.len();
        Self {
            futures: futures.into_iter().enumerate().collect(),
            results: (0..n).map(|_| None).collect(),
            completed: 0,
        }
    }
}

impl<F, T> RuyiFuture for JoinAll<F, T>
where
    F: RuyiFuture<Output = T> + Unpin,
{
    type Output = Vec<T>;

    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        for (idx, fut) in &mut self.futures {
            if self.results[*idx].is_none() {
                match fut.poll(waker) {
                    Poll::Ready(val) => {
                        self.results[*idx] = Some(val);
                        self.completed += 1;
                    }
                    Poll::Pending => {}
                }
            }
        }
        if self.completed == self.results.len() {
            Poll::Ready(
                self.results
                    .iter_mut()
                    .map(|o| o.take().unwrap())
                    .collect::<Vec<T>>(),
            )
        } else {
            Poll::Pending
        }
    }
}

/// Return the result of the first future to complete.
pub struct Race<F> {
    futures: Vec<F>,
}

impl<F> Race<F>
where
    F: RuyiFuture,
{
    pub fn new(futures: Vec<F>) -> Self {
        Self { futures }
    }
}

impl<F> RuyiFuture for Race<F>
where
    F: RuyiFuture + Unpin,
{
    type Output = F::Output;

    fn poll(&mut self, waker: &Waker) -> Poll<Self::Output> {
        for fut in &mut self.futures {
            match fut.poll(waker) {
                Poll::Ready(val) => return Poll::Ready(val),
                Poll::Pending => {}
            }
        }
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let waker = Waker {
            scheduler: SchedulerInner::new(1),
            worker_id: 0,
            task_id: TaskId(0),
        };
        assert_eq!(join.poll(&waker), Poll::Ready(vec![1, 2, 3]));
    }

    #[test]
    fn test_race() {
        let futs = vec![ImmediateFuture(Some(1)), ImmediateFuture(Some(2))];
        let mut race = Race::new(futs);
        let waker = Waker {
            scheduler: SchedulerInner::new(1),
            worker_id: 0,
            task_id: TaskId(0),
        };
        assert_eq!(race.poll(&waker), Poll::Ready(1));
    }

    #[test]
    fn test_yielding_future() {
        let mut fut = YieldingFuture { yielded: false };
        let waker = Waker {
            scheduler: SchedulerInner::new(1),
            worker_id: 0,
            task_id: TaskId(0),
        };
        assert_eq!(fut.poll(&waker), Poll::Pending);
        assert_eq!(fut.poll(&waker), Poll::Ready(42));
    }

    #[test]
    fn test_task_id_unique() {
        let scheduler = SchedulerInner::new(1);
        let a = scheduler.next_id();
        let b = scheduler.next_id();
        assert_ne!(a, b);
    }
}
