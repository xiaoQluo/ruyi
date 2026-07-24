# Proposal: add-test-runner-cli

## Why

`v0.5.8-stdlib-core` introduced the `@test fn` attribute and `crates/ruyic/src/runtime/test_registry.rs` provides `TestFunctionRegistry::collect_from_program` to discover annotated functions. **The runtime registers tests but never executes them.** No `ruyic --test` CLI flag exists, no `Driver::run_tests()` method exists, no test execution harness exists.

Consequence: a developer writes `tests/buffer.ry` with `@test fn push_then_pop() { ... }` — currently `target/release/ruyic tests/buffer.ry` produces a binary, not a test report. Developers must hand-instrument execution.

`docs/roadmap.md:285-286` already lists `10.1` `ruyi test runner` and `10.2` `@test attribute` as planned (target Q4 2027 / v1.1). This change ships them early as part of v0.6 prep.

## Root Cause

Three missing pieces:

| Missing | Today |
|---------|-------|
| CLI flag | `main.rs:21-53` `Args` struct has no `test: bool`; emits Binary / Check / etc. only |
| Driver entry point | `Driver` struct (in `crates/ruyic/src/driver.rs`) has no `run_tests()` method; pipeline ends at codegen+link |
| Execution harness | No test runner module exists; collect_from_program only stores metadata, never executes |

The data collection half (`TestFunctionRegistry`) is already complete; the execution half is missing.

## What Changes

### File 1 — `crates/ruyic/src/main.rs`

Add CLI flag:
```rust
#[derive(Parser, Debug)]
struct Args {
    // ... existing fields ...
    #[arg(long, help = "Run @test fn functions as tests")]
    test: bool,
}
```

In `fn main()` (line 55), change the emit-type computation:
```rust
let emit = if args.test {
    EmitType::Test
} else if args.emit_ast { EmitType::Ast }
// ...
```

### File 2 — `crates/ruyic/src/driver.rs`

Add `EmitType::Test` to the enum (around line 70-90). Add `Driver::run_tests()` method:
```rust
pub fn run_tests(&mut self, source: &Path) -> Result<TestReport, CompileError> {
    // 1. Lex + parse + typecheck (same as for binary)
    // 2. Collect @test fn entries
    let registry = TestFunctionRegistry::new();
    let program = /* re-use inferred */;
    registry.collect_from_program(&items, source.display().to_string(), module);
    // 3. Codegen each @test fn as an LLVM function with C-ABI:
    //    - void ruyi_test_<hash>(int64_t* out_status, char** out_msg)
    //    - entry point: dispatch to test fn body, catch assertions
    // 4. Link into test_runner.ll with a `main` that iterates each
    //    ruyi_test_* symbol, calls it, records status, prints PASS/FAIL
    // 5. Return TestReport { passed, failed, total, by_name: BTreeMap }
}
```

### File 3 — `crates/ruyic/src/test_runner.rs` (NEW, ~250 LOC)

LLVM IR-level test harness:
- Generates a `main` function:
  - Iterate over `ruyi_test_*` symbols (collected in `Driver::run_tests`)
  - For each: invoke, capture exit code / exception type
  - Print `PASS  <name>` or `FAIL  <name>: <message>`
  - Final exit code: 0 if all pass, 1 if any fail
- Re-uses `stdlib/test.ry` `assert_eq` / `assert_true` etc — no custom runner code; assertions throw `AssertionError`, the harness catches it via exception-unwinder code already in `crates/ruyic/src/codegen/stmt.rs:compile_try`.

### File 4 — `crates/ruyic/src/runtime/test_registry.rs`

Extend with:
```rust
pub fn run(&self, runner: &mut TestRunner) -> TestReport { ... }
```
(currently only collects; this just threads through to `TestRunner::run_test(id)`)

### File 5 — `crates/ruyic/src/lib.rs` + `cli` entry

Export `pub use crate::test_runner::{TestRunner, TestReport}` so `main.rs` can call `Driver::run_tests()` and `test_runner::run_report(report)`.

### File 6 — `examples/test_demo.ry` (NEW)

```ruyi
import { assert_eq, assert_true } from "./test";

@test fn arithmetic_holds() {
    assert_eq(1 + 1, 2);
    assert_eq(2 * 3, 6);
}

@test fn string_concat() {
    assert_eq("hello" + " " + "world", "hello world");
}

fn main() { print("main runs"); }
```

### File 7 — `tests/integration/test_runner.rs` (NEW, ~50 LOC)

Tests:
- `discovers_annotated_fns`: parse file with 2 `@test fn` + 1 plain `fn`; registry has exactly 2 entries.
- `exit_code_zero_on_all_pass`: compile and run `test_demo.ry --test`; expect exit 0; verify "PASS arithmetic_holds" on stdout.
- `exit_code_nonzero_on_assert_fail`: file with `@test fn failing() { assert_eq(1, 2); }`; expect exit 1 and "FAIL failing" on stdout.
- `non_test_fn_not_executed`: file with `@test fn passes() { assert_true(true); } fn helper() { print("HI"); }`; ensure "HI" never printed.

## Acceptance Criteria

1. `make check` / `make build-release` pass with no new warnings.
2. `./target/release/ruyic --test examples/test_demo.ry`:
   - Exits 0
   - Prints `PASS  arithmetic_holds` and `PASS  string_concat`
   - Does NOT print `main runs` (main is skipped in test mode)
3. `./target/release/ruyic --test /tmp/failing.ry` (a hand-written `@test fn` that asserts `1 === 2`):
   - Exits 1
   - Prints `FAIL  failing`
4. CLI flag is `--test` (kebab case, not `--tests`).
5. New integration tests pass.

## Scope (in)

- `crates/ruyic/src/main.rs` (add `--test`)
- `crates/ruyic/src/driver.rs` (add `run_tests`)
- `crates/ruyic/src/test_runner.rs` (NEW, ~250 LOC)
- `crates/ruyic/src/runtime/test_registry.rs` (extend with `run`)
- `crates/ruyic/src/lib.rs` (re-exports)
- `examples/test_demo.ry` (NEW)
- `crates/ruyic/tests/integration/test_runner.rs` (NEW)

## Scope (out / Scope Fence)

- ❌ Parallel test execution (sequential is sufficient; parallelism is v1.1+)
- ❌ Test name filter (`--filter <substr>`) — v1.1+
- ❌ Test reporting formats (JSON / JUnit-XML) — v1.1+
- ❌ `--bench` mode — separate change
- ❌ Test explorer IDE integration — defer to v1.1+
- ❌ Moving stdlib/test.ry assertion helpers (assert_eq, assert_true, etc.) into the runtime — they stay in stdlib.

## Impact

| Dimension | Impact |
|-----------|--------|
| Compiler binary | +5-10% (test runner embedded) |
| Compile time | unchanged (test harness is small) |
| User experience | first-class test runner; matches `cargo test`, `go test`, `pytest` UX |
| stdlib e2e | unchanged |
| `cargo test` count | +4 integration tests |
| ABI | new symbol `ruyi_test_runner_main` (entry point for test mode) |

## Capabilities (CLOSED)

- `ruyic-test-runner`: end-to-end `@test fn` execution with PASS/FAIL output
- `exit-code-convention`: 0 = all pass, 1 = any failure (matches Cargo)
- `assertion-integration`: stdlib/test.ry `assert_eq` / `assert_true` / `assert_not_null` / `assert_false` produce visible FAIL lines
