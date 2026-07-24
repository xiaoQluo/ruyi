# Execution Contract: stdlib-io-path-process-ffi

**Change**: `stdlib-io-path-process-ffi`
**Mode**: full
**State**: planned → bridging

## Intent Lock

为 `stdlib/io.ry`、`stdlib/path.ry`、`stdlib/process.ry` 三个系统级模块补齐缺失的 45 个 C FFI 后端符号——在 `crates/ruyi_runtime/src/` 新增 3 个 Rust FFI 源文件（`io_ffi.rs` 17 函数、`path_ffi.rs` 8 函数、`process_ffi.rs` 20 函数）、在 `crates/ruyic/src/codegen/builtins_table.rs` 添加 45 条 `BuiltinDecl` 声明、在 `lib.rs` 注册模块，并通过 Rust 单元测试 + `.ry` 集成测试 + `make build-release/test/lint/fmt-check` 全验证。遵循项目现有 FFI 模式（`#[no_mangle] pub extern "C"` + `BuiltinSig` 表驱动 + `ruyi_throw` 异常 + `ruyi_alloc` 内存），不引入新依赖，不改 stdlib `.ry` 源码。

## Affected Scope

### Source code (6 files)

| File | Change | Content |
|------|--------|---------|
| `crates/ruyi_runtime/src/path_ffi.rs` | **Create** | 8 `extern "C"` path functions (join/basename/dirname/extname/is_absolute/normalize/separator/relative) + `#[cfg(test)]` |
| `crates/ruyi_runtime/src/io_ffi.rs` | **Create** | 17 `extern "C"` I/O functions (9 sync: read_line/read_text/write_text/read_lines/exists/is_directory/is_file/delete/mkdir + 8 async variants) + `#[cfg(test)]` |
| `crates/ruyi_runtime/src/process_ffi.rs` | **Create** | 20 `extern "C"` process functions (exec/exec_with/create/wait/wait_async/kill + write_input/close_input/read_output/read_error + get_env/set_env/get_all_env + get_pid/get_ppid/get_platform/get_cpu_count/get_total_memory/get_free_memory + signal_available) + `#[cfg(test)]` |
| `crates/ruyi_runtime/src/lib.rs` | Modify | Add `pub mod path_ffi;`, `pub mod io_ffi;`, `pub mod process_ffi;` |
| `crates/ruyic/src/codegen/builtins_table.rs` | Modify | Add 45 `BuiltinDecl` entries (path:8, io:17, process:20); update header comment; update count test 56→64→81→101 |
| `crates/ruyic/tests/integration/path_test.ry` | **Create** | Path integration test |
| `crates/ruyic/tests/integration/io_test.ry` | **Create** | IO integration test |
| `crates/ruyic/tests/integration/process_test.ry` | **Create** | Process integration test |

### Planning artifacts

- `changes/stdlib-io-path-process-ffi/proposal.md`
- `changes/stdlib-io-path-process-ffi/specs/01-io-ffi.md`
- `changes/stdlib-io-path-process-ffi/specs/02-path-ffi.md`
- `changes/stdlib-io-path-process-ffi/specs/03-process-ffi.md`
- `changes/stdlib-io-path-process-ffi/design.md`
- `changes/stdlib-io-path-process-ffi/tasks.md`
- `changes/stdlib-io-path-process-ffi/execution-contract.md` (this file)
- `changes/stdlib-io-path-process-ffi/.spec-superflow.yaml`

## Task Batches (4 batches, in order)

| ID | Batch | Files | Functions | TDD pattern |
|----|-------|-------|-----------|-------------|
| B1 | Path FFI | 4 tasks: path_ffi.rs → builtins_table (8) → lib.rs → path_test.ry | 8 | RED (12 tests) → GREEN (implement) → REFACTOR (extract helpers) → VERIFY → DOC |
| B2 | IO FFI | 5 tasks: io_ffi.rs sync → io_ffi.rs async → builtins_table (17) → lib.rs → io_test.ry | 17 | RED (11+ sync tests + 2 async tests) → GREEN → REFACTOR → VERIFY → DOC |
| B3 | Process FFI | 6 tasks: process_ffi.rs exec+lifecycle → io pipes → env+signal → builtins_table (20) → lib.rs → process_test.ry | 20 | RED (9+4+6 tests, progressive) → GREEN → REFACTOR → VERIFY → DOC |
| B4 | Final Verify | 6 tasks: check → test → build-release → lint+fmt → stdlib regression → dp_5 | — | Verify-only: `cargo check --workspace`, `cargo test --workspace`, `make build-release`, `make lint`, `make fmt-check`, all `stdlib/*.ry --check` |

**Total estimated time**: ~3.5 hours（B1 45min + B2 70min + B3 90min + B4 20min）

**builtins_table count progression**: 56 (current) → 64 (B1) → 81 (B2) → 101 (B3 final)

## Approved Behavior

After all 4 batches complete:

1. `libruyi_runtime.a` SHALL contain `io_ffi.o`, `path_ffi.o`, `process_ffi.o` with all 45 `__io_*`/`__path_*`/`__process_*` symbols
2. `import { File } from "./io"` SHALL compile, link, and execute without undefined symbol errors
3. `import { Path } from "./path"` SHALL compile, link, and execute without undefined symbol errors
4. `import { Process, getEnv, getPID } from "./process"` SHALL compile, link, and execute without undefined symbol errors
5. All 3 `.ry` integration tests SHALL pass: `path_test`, `io_test`, `process_test`
6. All pre-existing tests SHALL continue to pass (no regressions)
7. `make lint` SHALL show zero new clippy warnings
8. `make fmt-check` SHALL pass
9. All 14 `stdlib/*.ry` files SHALL pass `ruyic --check`

## Out of Scope (Fence)

- Do NOT modify `stdlib/io.ry`, `stdlib/path.ry`, `stdlib/process.ry` or any other `.ry` file
- Do NOT add new stdlib modules beyond IO/Path/Process
- Do NOT refactor codegen pipeline or driver.rs
- Do NOT implement Windows support
- Do NOT add external dependencies (use Rust stdlib only)
- Do NOT fix pre-existing issues (clippy warnings, test failures, `json.ry` `!=` bug)
- Do NOT change the async runtime architecture

## Build Rules

1. All `extern "C"` functions MUST use `#[no_mangle]`
2. All FFI functions MUST follow existing naming convention (`__module_function`)
3. All `BuiltinDecl` entries MUST use correct `BuiltinSig` mapping:
   - `string` → `BuiltinSig::String` (input) or `BuiltinSig::Ptr` (opaque handles)
   - `int` → `BuiltinSig::Int`
   - `float` → `BuiltinSig::Float`
   - `bool` → `BuiltinSig::Bool`
   - `void` → `BuiltinSig::Void` (return only)
4. Memory allocation for return strings MUST use `ruyi_alloc` (or compatible malloc)
5. Error reporting MUST use `ruyi_throw` (not return codes, not panic)
6. Process handle lifecycle: `__process_create` → `Box::into_raw` → opaque pointer, `__process_wait`/`__process_kill` → `Box::from_raw` → cleanup
7. Platform-specific code MUST use `#[cfg(not(target_os = "windows"))]` guards

## Review Gates

| Gate | After | Check |
|------|-------|-------|
| G1 | B1 complete | `cargo test -p ruyi_runtime -- path_ffi` passes 12 tests; `cargo test -p ruyic -- builtins_table::builtins_count_is_64` passes |
| G2 | B2 complete | `cargo test -p ruyi_runtime -- io_ffi` passes 13+ tests; `cargo test -p ruyic -- builtins_table::builtins_count_is_81` passes |
| G3 | B3 complete | `cargo test -p ruyi_runtime -- process_ffi` passes 19+ tests; `cargo test -p ruyic -- builtins_table::builtins_count_is_101` passes |
| G4 | B4 complete | All acceptance criteria verified (see Approved Behavior) |

## Handoff Rules

1. Each batch MUST be verified at its gate before starting the next batch
2. If any pre-existing test breaks: STOP, document the failure, escalate
3. If `make lint` shows new clippy warnings: fix before proceeding to next batch
4. If `make fmt-check` fails: run `make fmt` before proceeding
5. After G4: record `dp_5_result` in `.spec-superflow.yaml` with verification evidence
6. Do NOT merge, tag, or release unless explicitly requested by user

## Escalation Rules

| Condition | Action |
|-----------|--------|
| New test fails without clear cause | Run `cargo test -- --nocapture`, check error message, fix root cause. If >3 attempts: consult Oracle |
| Pre-existing test regression | Document which test, verify it's unrelated to this change, continue |
| `make build-release` fails | Check LLVM 14 availability (`brew list llvm@14`), check `LLVM_SYS_140_PREFIX` env var |
| Integration `.ry` test fails to compile | Check `builtins_table.rs` entry signature matches stdlib `.ry` function signature exactly |
| Process pipe test hangs | Check for deadlock in stdin/stdout pipe handling, add timeout to test |
| `ruyi_throw` not found in io_ffi/process_ffi | Verify `use ruyi_runtime::exception::runtime::ruyi_throw;` import |
| `__builtin_map_create`/`__builtin_array_push` not found in process_ffi | Verify these are `pub` in `builtins.rs` and process_ffi has access |
