# Spec: `ruyi_await` Real Async Implementation

## MODIFIED Requirements

### REQ-AWAIT-001: `ruyi_await` MUST actually suspend and resume the current coroutine

The `ruyi_await` runtime function MUST:
1. Check whether the awaited future is ready
2. If ready, return the result
3. If not ready, suspend the current coroutine and yield control to the scheduler
4. When the future becomes ready, resume the coroutine with the result

Currently, `ruyi_await` is a no-op stub. This spec replaces it with a real implementation.

#### Scenario: Awaiting a ready future returns immediately
- **WHEN** source contains `let result = await readyFuture();` where `readyFuture()` returns an immediately-ready future
- **THEN** `result` holds the future's value and execution continues without suspension

#### Scenario: Awaiting a pending future suspends and resumes
- **WHEN** source contains `let result = await pendingFuture();` where `pendingFuture()` becomes ready after a delay
- **THEN** the coroutine suspends, the scheduler runs other coroutines, and `result` is assigned when the future resolves

### REQ-AWAIT-002: Work-stealing scheduler MUST be present in `ruyi_runtime`

`ruyi_runtime` MUST contain a work-stealing scheduler that maintains a queue of runnable coroutines per worker thread and steals from other workers when its own queue is empty.

#### Scenario: Scheduler has multiple worker threads
- **WHEN** the runtime is initialized
- **THEN** the scheduler spawns N worker threads (N = `num_cpus::get()` by default)

#### Scenario: Coroutine migration between workers
- **WHEN** worker A's queue is empty and worker B has runnable coroutines
- **THEN** worker A steals one coroutine from worker B and resumes it

### REQ-AWAIT-003: Async examples MUST compile and run

A new async example `examples/async_sleep.ry` MUST demonstrate working `await`:

```ruyi
async fn main(): int {
  print("before");
  await sleep(100);
  print("after");
  return 0;
}
```

#### Scenario: Async example runs successfully
- **WHEN** `examples/async_sleep.ry` is compiled with `ruyic --gc=real` and run
- **THEN** the program prints both "before" and "after" with the delay in between