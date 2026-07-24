# Tasks: fix-exception-unwinder

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/ruyi_runtime/src/exception.rs` | Modify | `throw_exception`: `panic!()` → 调用 `exception/runtime.rs:ruyi_throw` |
| `crates/ruyi_runtime/src/c_exports.rs` | Modify | C FFI `ruyi_throw`: 存储指针 → 构建 ExceptionObject + 调用 `throw_exception` |
| `crates/ruyic/src/runtime/exception.rs` | Modify | `Exception::throw`: `panic!()` → 同步使用 runtime 路径 |
| `crates/ruyi_runtime/tests/exception_runtime.rs` | Modify | 移除 4 个 `#[ignore]`，验证测试通过 |
| `crates/ruyic/tests/integration/cases/codegen/exception_throw.ry` | Create | 端到端：throw→catch |
| `crates/ruyic/tests/integration/cases/codegen/exception_throw.expected` | Create | 期望输出 |
| `crates/ruyic/tests/integration/cases/codegen/exception_nested.ry` | Create | 端到端：嵌套 try-catch-finally |
| `crates/ruyic/tests/integration/cases/codegen/exception_nested.expected` | Create | 期望输出 |
| `crates/ruyic/tests/integration/cases/codegen/exception_rethrow.ry` | Create | 端到端：catch 内 rethrow |
| `crates/ruyic/tests/integration/cases/codegen/exception_rethrow.expected` | Create | 期望输出 |
| `crates/ruyic/tests/codegen.rs` | Modify | 添加 3 个 fixture-based 测试函数 |

## Interfaces

### Cross-Module

| Producer | Consumer | Interface |
|----------|----------|-----------|
| `exception.rs:throw_exception` | `exception/runtime.rs:ruyi_throw` | `throw_exception` 包装 `RuyiException` → `ExceptionObject`，unsafe 调用 `ruyi_throw(ptr)` |
| `c_exports.rs:ruyi_throw` | `exception.rs:throw_exception` | C FFI 构建 `RuyiException`，调用 `throw_exception` |
| Codegen `compile_throw` | Runtime `ruyi_throw` (C FFI) | 通过 LLVM `build_call` 调用 `#[no_mangle] ruyi_throw` 符号 |

---

## Batch 1: Runtime Unwinder Connection [Independent]

### Task 1.1: Connect throw_exception to ruyi_throw
- **File**: `crates/ruyi_runtime/src/exception.rs` (line 286)
- **TDD**:
  1. 确认当前 `throw_exception` 用 `panic!()` 占位
  2. 实现：构建 `ExceptionObject` → `Box::into_raw` → unsafe 调用 `exception::runtime::ruyi_throw(ptr)`
  3. 添加 safety 注释说明 `ptr` 所有权转移给 unwinder（cleanup 回调负责释放）
  4. 运行 `cargo check -p ruyi_runtime` 确认编译通过
  5. 运行 `cargo test --lib -p ruyi_runtime` 确认已有测试通过（throw_exception 相关的单元测试若使用 `#[should_panic]`，需更新为其他验证方式）
- **Interfaces**:
  - Consumes: `exception::runtime::ruyi_throw(exception: *mut ExceptionObject) -> !`
  - Consumes: `ExceptionObject::new(type_id, message)`

### Task 1.2: Upgrade C FFI ruyi_throw
- **File**: `crates/ruyi_runtime/src/c_exports.rs` (line 9)
- **TDD**:
  1. 确认当前 `ruyi_throw` 仅存储 `PENDING_EXCEPTION`
  2. 实现：从 `msg: *const i8` 读取 CStr → 构建 `RuyiException` → 调用 `throw_exception`
  3. 保留 `PENDING_EXCEPTION` 和 `ruyi_get_pending_exception` 等辅助函数（可能仍有其他调用方）
  4. 运行 `cargo check -p ruyi_runtime` 确认编译通过（注意 `throw_exception` 返回 `!`——编译器会识别后续代码为 dead code）
- **Interfaces**:
  - Consumes: `exception::throw_exception(RuyiException) -> !`
  - Consumes: `std::ffi::CStr::from_ptr`

### Task 1.3: Audit and retire PENDING_EXCEPTION usage
- **File**: `crates/ruyi_runtime/src/c_exports.rs`
- **Work**:
  1. 搜索 `PENDING_EXCEPTION` / `ruyi_get_pending_exception` / `ruyi_clear_pending_exception` 的所有引用
  2. 若仅在 `compile_throw` return-based fallback 中使用，标记为 deprecated 并添加 `#[allow(dead_code)]`
  3. 若已无调用方：保留符号以防 ABI 断裂，但内部实现改为 no-op
- **Depends on**: Task 1.2（了解 `compile_throw` 是否仍依赖 pending exception 机制）

---

## Batch 2: Compiler-Side Throw Update [Independent]

### Task 2.1: Update Exception::throw in ruyic runtime
- **File**: `crates/ruyic/src/runtime/exception.rs` (line 15)
- **TDD**:
  1. 确认当前 `Exception::throw()` 用 `panic!("{}: {}", ...)` 占位
  2. 替换为：保留结构但使用 `std::process::abort()` 作为编译器侧 throw 的实现
  3. 或：移除 `Exception::throw()` 方法，因为编译后的代码通过 codegen 直接调用 runtime FFI `ruyi_throw`，不经过 ruyic 的 `Exception::throw`
  4. 运行 `make check` 确认编译通过
  5. 确认无引用 `Exception::throw` 的调用方（grep 验证）
- **Interfaces**: 无外部接口——此文件是 ruyic crate 内部使用

### Task 2.2: Verify compile_throw codegen correctness
- **File**: `crates/ruyic/src/codegen/stmt.rs` (line 718)
- **Work**:
  1. 审查 `compile_throw`：确认调用 `ruyi_throw` 后的 control flow 在 unwinder 模式下的行为
  2. 关键检查点：
     - try 块内 throw（line 723-738）：`ruyi_throw` 调用后 `build_unconditional_branch(catch_bb)`，但 unwinder 会直接跳转到 landing pad——分支是否为 dead code？
     - try 块外 throw（line 740-755）：`ruyi_throw` 调用后 `build_return(None)`——但 `ruyi_throw` 现在是 `-> !`，return 为 dead code
  3. 方案：在 `ruyi_throw` 调用后添加 `build_unreachable()`（告诉 LLVM 此路径不可达）
  4. 运行 `make check` 确认编译通过
- **Depends on**: Task 1.2（C FFI `ruyi_throw` 变为 `-> !`）

---

## Batch 3: Test Enablement & Verification [Depends on: Batch 1, Batch 2]

### Task 3.1: Enable ignored exception_runtime tests
- **File**: `crates/ruyi_runtime/tests/exception_runtime.rs`
- **TDD**:
  1. 逐个审查 4 个 `#[ignore]` 测试，确认修复后的行为与测试期望一致：
     - `test_ruyi_throw_aborts_when_unwind_returns` (line 45): 预期 abort——需 `#[should_panic]` 或特殊测试配置
     - `test_ruyi_end_catch_no_panic_without_active_catch` (line 124): 需要 LLVM exception 基础设施
     - `test_function_exception_table_multiple_entries` (line 258): 纯数据结构测试——应直接通过
     - `test_nested_try_catch_propagates_to_outer` (line 528): 纯数据结构测试——应直接通过
  2. 对于需要 abort 行为的测试：使用 `#[should_panic]` 或测试框架的 abort 捕获机制
  3. 对于需要 LLVM 基础设施的测试：如无法在纯 Rust 测试中模拟，保留 `#[ignore]` 并添加说明
  4. 运行 `cargo test -p ruyi_runtime --test exception_runtime` 验证
- **Depends on**: Task 1.1, 1.2

### Task 3.2: Verify try_catch_invoke tests
- **File**: `crates/ruyic/tests/try_catch_invoke.rs`（12 个测试）
- **TDD**:
  1. 运行 `cargo test -p ruyic --test try_catch_invoke` — 记录当前通过/失败状态
  2. 对于使用 `throw Error.new(...)` 模式但因 "Complex new expressions" 限制而失败的测试：确认 Change B 修复后此处的修复不再需要额外工作
  3. 对于因 unwinder 行为变化而失败的测试：逐个调试，更新期望或修复 codegen
  4. 目标：12/12 通过
- **Depends on**: Task 2.2, Change B (Task 3b.3: complex new expressions)

### Task 3.3: Add end-to-end .ry integration test: basic throw→catch
- **Files**:
  - `crates/ruyic/tests/integration/cases/codegen/exception_throw.ry` (Create)
  - `crates/ruyic/tests/integration/cases/codegen/exception_throw.expected` (Create)
  - `crates/ruyic/tests/codegen.rs` (Modify: add test function)
- **TDD**:
  1. 创建 `.ry` 源文件：
     ```
     fn main() {
       try {
         throw Error.new("test error message");
       } catch (e: Error) {
         print(e.getMessage());
       }
       print("done");
     }
     ```
  2. 创建 `.expected` 文件：`test error message\ndone`
  3. 在 `codegen.rs` 中添加 `codegen_fixture_exception_throw` 测试函数（参考现有 fixture 测试模式）
  4. 运行测试确认通过
  5. 运行 `cargo test -p ruyic --test codegen` 确认无回归
- **Depends on**: Task 2.2, Change B (Task 3b.3)

### Task 3.4: Add end-to-end .ry integration test: nested try-catch-finally
- **Files**:
  - `crates/ruyic/tests/integration/cases/codegen/exception_nested.ry` (Create)
  - `crates/ruyic/tests/integration/cases/codegen/exception_nested.expected` (Create)
  - `crates/ruyic/tests/codegen.rs` (Modify: add test function)
- **TDD**:
  1. 创建 `.ry` 源文件：外层 try→catch + 内层 try→finally（rethrow 到外层）
  2. 创建 `.expected` 文件
  3. 添加测试函数
  4. 运行测试确认通过
- **Depends on**: Task 3.3

### Task 3.5: Add end-to-end .ry integration test: catch rethrow
- **Files**:
  - `crates/ruyic/tests/integration/cases/codegen/exception_rethrow.ry` (Create)
  - `crates/ruyic/tests/integration/cases/codegen/exception_rethrow.expected` (Create)
  - `crates/ruyic/tests/codegen.rs` (Modify: add test function)
- **TDD**:
  1. 创建 `.ry` 源文件：catch 块内 throw 新异常
  2. 创建 `.expected` 文件
  3. 添加测试函数
  4. 运行测试确认通过
- **Depends on**: Task 3.3

---

## Final Verification Wave

### Task FV.1: make check
```bash
make check
```
预期：通过，零新增 clippy 警告

### Task FV.2: Runtime test suite
```bash
cargo test -p ruyi_runtime --test exception_runtime
```
预期：全部通过（含 4 个 previously-ignored，或对无法测试的保留 ignore 并注释原因）

### Task FV.3: try_catch_invoke test suite
```bash
cargo test -p ruyic --test try_catch_invoke
```
预期：12/12 通过

### Task FV.4: Codegen test suite (including new fixtures)
```bash
cargo test -p ruyic --test codegen
```
预期：全部通过（含 3 个新 fixture 测试），零回归

### Task FV.5: Full workspace test
```bash
cargo test --workspace
```
预期：全部通过（允许 pre-existing GC clippy warnings）

### Task FV.6: Manual throw→catch verification
```bash
cargo run -- compile-file examples/hello.ry -o /tmp/test_hello
# 创建一个简单的 throw→catch .ry 文件
# 编译并运行，验证异常被正确捕获
```
预期：编译的 Ruyi 程序 throw→catch 正常工作，无 Rust panic backtrace
