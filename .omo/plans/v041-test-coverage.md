# v0.4.1 Complete Test Coverage Plan

## TL;DR

> **Quick Summary**: Add comprehensive integration tests and runtime unit tests for all 7 missing v0.4.1 feature test gaps, including edge cases and cross-feature integration tests.
>
> **Deliverables**:
> - 7 integration `.ry` test files (for-in, for-of, optional chaining, computed member, template literals, impl trait, match edge cases)
> - 3 runtime unit test files (thread-local GC, async GC roots, exception landing pads)
> - 5+ edge case test scenarios
>
> **Estimated Effort**: Medium
> **Parallel Execution**: YES — 2 waves
> **Critical Path**: Wave 1 (all parallel) → Wave 2 (cross-feature depends on Wave 1)

---

## Context

### Original Request
用户要求增加测试案例，完成所有 v0.4.1 特性的测试覆盖。

### Gap Analysis
Existing test infrastructure covers basic loops, match, exceptions, async, BigInt, and member access. Missing coverage for:
- for-in/for-of loops (codegen feature T1)
- Optional chaining `?.` (codegen feature T3)
- Computed member `obj[expr]` (codegen feature T3)
- Template literals (codegen feature T4)
- `impl Trait for` built-in types (typechecker feature T10)
- Thread-local GC heap (runtime feature T12)
- Async GC roots for GenerationalCollector (runtime feature T9)
- Edge cases and cross-feature combinations

### Research Findings
- No LLVM 14 available → integration tests use `--emit-llvm` + grep for codegen, `--check` for typecheck
- Runtime tests: `cargo test -p ruyi_runtime --no-default-features` (91 already passing)
- Existing test patterns: `tests/integration/runner.rs` with `.ry` + `.expected` pairs

---

## Work Objectives

### Core Objective
Add comprehensive test coverage for all 7 missing v0.4.1 feature test gaps.

### Concrete Deliverables
| Type | File | Covers Feature |
|------|------|----------------|
| Integration | `test/for_in.ry` + `.expected` | for-in loop |
| Integration | `test/for_of.ry` + `.expected` | for-of loop |
| Integration | `test/optional_chaining.ry` | Optional chaining |
| Integration | `test/computed_member.ry` | Computed member |
| Integration | `test/template_literal.ry` | Template literals |
| Integration | `test/impl_trait_builtin.ry` | impl Trait for built-in |
| Integration | `test/match_edge_cases.ry` | Match edge cases |
| Runtime | `crates/ruyi_runtime/tests/gc_thread_local.rs` | Thread-local GC |
| Runtime | `crates/ruyi_runtime/tests/gc_async_roots.rs` | Async GC roots |
| Runtime | `crates/ruyi_runtime/tests/exception_runtime.rs` | Exception landing pads |

> **目录约束**: 所有 `.ry` 文件统一放在 `test/` 目录，编译输出统一放在 `test/target/` 目录。

### Definition of Done
- [ ] 7 new .ry integration test files with expected output
- [ ] 3 new runtime unit test files
- [ ] 5+ edge case scenarios included
- [ ] `cargo test -p ruyi_runtime --no-default-features` passes (all new + existing 91)
- [ ] `ruyic --check` passes on all new .ry files
- [ ] `ruyic --emit-llvm` verified for codegen tests (where applicable)

### Must Have (硬性约束)
- **MUST**: 所有 .ry 测试文件放入 `test/` 目录（顶层目录，非 `tests/`）
- **MUST**: 编译输出文件放入 `test/target/` 目录
- **MUST**: 测试前确保 `test/` 和 `test/target/` 目录已创建

### Must NOT Have (禁止事项)
- **MUST NOT**: 将 .ry 文件放入 `tests/integration/cases/` 或 `examples/`
- **MUST NOT**: 编译输出放系统 `/tmp` 目录
- **MUST NOT**: 修改已有测试文件或已有源码
- **MUST NOT**: 引入新外部依赖

---

## Verification Strategy

### Test Decision
- **Infrastructure exists**: YES (cargo test + integration runner + --emit-llvm)
- **Automated tests**: New tests are the deliverable
- **Framework**: Rust `#[test]` + `.ry` integration tests

### No-LLVM Protocol
- Codegen tests: `ruyic test.ry --emit-llvm` → grep for expected IR patterns
- Type-check tests: `ruyic test.ry --check` → verify exit code 0
- Error tests: `ruyic test.ry --check` → verify exit code != 0, check error message

### QA Policy
Each task includes:
1. Run `ruyic --check` on the new .ry file
2. Run `ruyic --emit-llvm` and grep for key patterns (codegen tests)
3. Run `cargo test` on new runtime tests
4. Save evidence to `.sisyphus/evidence/`

---

## Execution Strategy

### Parallel Execution Waves

```
Prerequisite (START HERE — directory setup):
└── T0: Create test/ and test/target/ directories [quick]

Wave 1 (After T0 — isolated tests, MAX PARALLEL):
├── T1: for-in loop integration test [quick]
├── T2: for-of loop integration test [quick]
├── T3: Optional chaining integration test [quick]
├── T4: Computed member integration test [quick]
├── T5: Template literal integration test [quick]
├── T6: impl Trait for built-in integration test [quick]
├── T7: Match edge cases integration test [quick]
├── T8: Thread-local GC runtime tests [quick]
├── T9: Async GC roots runtime tests [quick]
└── T10: Exception landing pad runtime tests [quick]

Wave 2 (Cross-feature + edge cases, depends on Wave 1):
├── T11: Cross-feature integration tests [quick]
└── T12: Summary and coverage report [quick]
```

Critical Path: T0 → Wave 1 → Wave 2
Max Concurrent: 10 (Wave 1)

---

## TODOs

- [x] 0. **创建 test/ 和 test/target/ 目录** (前置)

  **What to do**:
  - 创建 `test/` 目录（存放 .ry 测试文件）
  - 创建 `test/target/` 目录（存放编译输出）
  - 添加到 `.gitignore`：`test/target/`（编译产物不入库）
  - 提交目录结构

  **Must NOT do**:
  - 不要删除已有的 `tests/` 目录
  - 不要修改已有测试文件

  **Commit**: YES | `chore: create test/ and test/target/ directories`

- [x] 1. **for-in loop integration test** (P0)

  **What to do**:
  - Create `test/for_in.ry`
  - Test: `let obj = {a: 1, b: 2, c: 3}; for (let k in obj) { print(k); }`
  - Create `test/for_in.expected` with expected output: `a\nb\nc`
  - Compile output to `test/target/for_in`
  - Verify with `ruyic --check` and `ruyic --emit-llvm` looking for `ruyi_obj_keys`

  **QA Scenarios**:
  - Empty object: `for (let k in {}) { }` → no output, no crash
  - Nested for-in: inner loop keys print correctly

  **Evidence**: `.sisyphus/evidence/task-test-1-for-in.txt`

  **Commit**: YES | `test: add for-in loop integration tests`

- [x] 2. **for-of loop integration test** (P0)

  **What to do**:
  - Create `test/for_of.ry`
  - Test: `let arr = [10, 20, 30]; for (let item of arr) { print(item); }`
  - Create `for_of.expected` with expected output: `10\n20\n30`
  - Test iterator protocol: `let s = "hello"; for (let ch of s) { print(ch); }`

  **QA Scenarios**:
  - Empty array: no output
  - String iteration via for-of

  **Evidence**: `.sisyphus/evidence/task-test-2-for-of.txt`

  **Commit**: YES | `test: add for-of loop integration tests`

- [x] 3. **Optional chaining integration test** (P0)

  **What to do**:
  - Create `test/optional_chaining.ry`
  - Test null object: `let obj = null; print(obj?.prop);` → `null`
  - Test valid object: `let obj = {prop: "value"}; print(obj?.prop);` → `value`
  - Test deep chain: `let a = {b: null}; print(a?.b?.c);` → `null`
  - Verify `--emit-llvm` shows `icmp eq.*null` instructions

  **QA Scenarios**:
  - Null object → null result
  - Valid object → correct property
  - Deep chain short-circuit
  - Method-level optional (if supported)

  **Evidence**: `.sisyphus/evidence/task-test-3-opt-chain.txt`

  **Commit**: YES | `test: add optional chaining integration tests`

- [x] 4. **Computed member integration test** (P0)

  **What to do**:
  - Create `test/computed_member.ry`
  - Test dynamic key: `let key = "name"; let obj = {name: "Ruyi"}; print(obj[key]);`
  - Test numeric index: `let arr = ["a", "b", "c"]; let i = 1; print(arr[i]);`
  - Verify `--emit-llvm` shows `ruyi_obj_get` call

  **QA Scenarios**:
  - String key lookup
  - Integer index lookup
  - Invalid key (returns null)

  **Evidence**: `.sisyphus/evidence/task-test-4-computed.txt`

  **Commit**: YES | `test: add computed member integration tests`

- [x] 5. **Template literal integration test** (P1)

  **What to do**:
  - Create `test/template_literal.ry`
  - Test interpolation: `` let name = "Ruyi"; print(`Hello ${name}!`); `` → `Hello Ruyi!`
  - Test multiple expressions: `` let a = 1, b = 2; print(`${a} + ${b} = ${a + b}`); `` → `1 + 2 = 3`
  - Test pure string: `` print(`plain text`); `` → `plain text`
  - Verify `--emit-llvm` shows `ruyi_str_concat` calls

  **QA Scenarios**:
  - Single interpolation
  - Multiple interpolations
  - Pure string (no interpolation)
  - Empty template: `` ```` → empty string

  **Evidence**: `.sisyphus/evidence/task-test-5-template.txt`

  **Commit**: YES | `test: add template literal integration tests`

- [x] 6. **impl Trait for built-in integration test** (P1)

  **What to do**:
  - Create `test/impl_trait_builtin.ry`
  - Test string: `trait P { fn fmt(self): string; } impl P for string { fn fmt(self): string { return self; } } print("hello".fmt());`
  - Test int: `impl P for int { fn fmt(self): string { return "int"; } } print(42.fmt());`
  - Test float/bool impls
  - Verify `--check` passes (no "trait not implemented" errors)

  **QA Scenarios**:
  - string impl passes trait bounds
  - int impl passed to generic function
  - float and bool impls work

  **Evidence**: `.sisyphus/evidence/task-test-6-impl-trait.txt`

  **Commit**: YES | `test: add impl Trait for built-in integration tests`

- [x] 7. **Match edge cases integration test** (P1)

  **What to do**:
  - Create `test/match_edge_cases.ry`
  - Test nested match: `match x { 1 => match y { ... }, ... }`
  - Test match with return: `fn f(x: int): string { match x { 1 => return "one", _ => return "other" } }`
  - Test match on strings: `match s { "a" => 1, "b" => 2, _ => 0 }`
  - Test pattern binding with variable reuse (shadowing)
  - Verify `--emit-llvm` shows correct switch/br patterns

  **QA Scenarios**:
  - Nested match compiles
  - Match with return exits correctly
  - String match generates strcmp chain
  - Multiple arms with same body compile

  **Evidence**: `.sisyphus/evidence/task-test-7-match-edge.txt`

  **Commit**: YES | `test: add match edge case integration tests`

- [x] 8. **Thread-local GC runtime tests** (P1)

  **What to do**:
  - Create `crates/ruyi_runtime/tests/gc_thread_local.rs`
  - Test: alloc in one thread, verify objects survive thread lifetime
  - Test: thread exit correctly calls collector Drop
  - Test: `CURRENT_COLLECTOR.with()` pattern works
  - Follow existing `gc_exports.rs` test patterns

  **QA Scenarios**:
  - Thread-local allocation returns unique pointers
  - Collect on one thread doesn't affect another
  - Thread exit frees heap

  **Evidence**: `.sisyphus/evidence/task-test-8-gc-thread.txt`

  **Commit**: YES | `test: add thread-local GC runtime tests`

- [x] 9. **Async GC roots runtime tests** (P1)

  **What to do**:
  - Create `crates/ruyi_runtime/tests/gc_async_roots.rs`
  - Test: `register_async_roots` correctly scans Task objects
  - Test: GC doesn't collect objects referenced by async tasks
  - Test: task completion releases GC roots
  - Follow existing `async_runtime.rs` test patterns

  **QA Scenarios**:
  - Suspended task's references survive collection
  - Multiple tasks each have independent roots
  - Completed task's objects can be collected

  **Evidence**: `.sisyphus/evidence/task-test-9-async-roots.txt`

  **Commit**: YES | `test: add async GC roots runtime tests`

- [x] 10. **Exception landing pad runtime tests** (P1)

  **What to do**:
  - Create `crates/ruyi_runtime/tests/exception_runtime.rs`
  - Test: `ruyi_throw` correctly calls `_Unwind_RaiseException` (mock/stub test)
  - Test: `ruyi_begin_catch` returns correct exception
  - Test: `ruyi_end_catch` cleans up
  - Test: cross-function exception propagation
  - Follow existing `runtime.rs` test patterns

  **QA Scenarios**:
  - throw produces exception object
  - begin_catch + end_catch lifecycle
  - Nested try-catch in runtime context

  **Evidence**: `.sisyphus/evidence/task-test-10-exception.txt`

  **Commit**: YES | `test: add exception landing pad runtime tests`

- [x] 11. **Cross-feature integration tests** (P2)

  **What to do**:
  - Create combined test scenarios:
    - `for-in` + `break` inside optional chain context
    - `match` inside `try-catch`
    - Async fn with template literal and optional chaining
    - Template literal inside match arm
    - Computed member + optional chaining chain
  - Create `test/cross_features.ry`
  - Verify all compose without errors via `--check`

  **QA Scenarios**:
  - Match-inside-try compiles
  - Async + template literal works
  - for + break + optional chaining composes
  - All cross-feature combinations type-check

  **Evidence**: `.sisyphus/evidence/task-test-11-cross.txt`

  **Commit**: YES | `test: add cross-feature integration tests`

- [x] 12. **Summary and coverage report** (P2)

  **What to do**:
  - Generate coverage report showing all 11 features have at least 1 test
  - Run `cargo test -p ruyi_runtime --no-default-features` and verify all pass
  - Count total test cases (integration .ry + runtime unit tests)
  - Output report to `.sisyphus/evidence/coverage-report.md`

  **QA Scenarios**:
  - All 7 gap areas have tests
  - Existing 91 tests still pass
  - New tests all pass
  - Coverage report generated

  **Evidence**: `.sisyphus/evidence/coverage-report.md`

  **Commit**: YES | `test: generate v0.4.1 test coverage report`

---

## Final Verification Wave

- [x] F1. **Test Execution Audit** — `quick`
  Run ALL tests across the project. Verify 0 failures.
  Output: `Integration tests [N/N] | Runtime tests [N/N] | VERDICT`

- [x] F2. **Coverage Gap Check** — `quick`
  Map each v0.4.1 feature to at least one test. Flag any uncovered features.
  Output: `Features [11/11 covered] | Gaps [0] | VERDICT`

---

## Commit Strategy

- **G1** (Wave 1 complete): `test: add v0.4.1 integration and runtime test coverage`
- **G2** (Wave 2 complete): `test: add cross-feature tests and coverage report`

---

## Success Criteria

### Verification Commands
```bash
# All runtime tests pass
cargo test -p ruyi_runtime --no-default-features

# All integration .ry files type-check
for f in test/*.ry; do ruyic "$f" --check && echo "PASS" || echo "FAIL: $f"; done
```

### Final Checklist
- [ ] All 7 gap areas have at least 1 test
- [ ] New runtime tests: 3 files with 5+ test functions each
- [ ] New integration tests: 7+ .ry files with .expected or --check verification
- [ ] Existing 91 runtime tests still pass
- [ ] Cross-feature tests added
- [ ] Coverage report generated

