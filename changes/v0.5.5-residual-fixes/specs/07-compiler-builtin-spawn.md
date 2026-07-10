# Spec: `spawn` Builtin for Green Thread Creation

## ADDED Requirements

### REQ-SPAWN-001: `spawn(fn)` MUST start a new coroutine on the scheduler

The compiler MUST provide a built-in `spawn(fn: () => void): void` function that:
1. Allocates a new coroutine via `ruyi_gc_alloc`
2. Wraps `fn` as a future
3. Submits the future to the work-stealing scheduler
4. Returns immediately to the caller

#### Scenario: spawn runs the function asynchronously
- **WHEN** source contains `spawn(() => print("hello")); print("world");`
- **THEN** the program compiles, "world" prints before "hello" (or interleaved, depending on scheduler), and both eventually appear

#### Scenario: spawn with arguments
- **WHEN** source contains `spawn((x: int) => print(x))(42);`
- **THEN** the program compiles and eventually prints `42`

### REQ-SPAWN-002: `spawn` MUST be available in `--gc=real` mode only

`spawn` requires the runtime scheduler and GC, so it MUST only be callable when `--gc=real` is specified. With `--gc=stub`, `spawn` is a compile error.

#### Scenario: spawn in stub mode rejected
- **WHEN** source contains `spawn(() => print("x"));` and compiled with default `--gc=stub`
- **THEN** compiler prints error "spawn requires --gc=real" and exits non-zero

#### Scenario: spawn in real mode accepted
- **WHEN** source contains `spawn(() => print("x"));` and compiled with `--gc=real`
- **THEN** compilation succeeds

### REQ-SPAWN-003: A new example MUST demonstrate `spawn`

A new example `examples/spawn_demo.ry` MUST demonstrate `spawn` usage with at least 3 concurrent tasks.

#### Scenario: spawn example runs and produces interleaved output
- **WHEN** `examples/spawn_demo.ry` is compiled with `ruyic --gc=real` and run
- **THEN** the program prints output from at least 3 spawned tasks, with output interleaved (proving concurrency)