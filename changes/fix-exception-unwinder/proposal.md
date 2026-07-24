# Proposal: fix-exception-unwinder

## Why

Ruyi v0.5.9 的 `throw`/`try`/`catch` 在编译后的程序中**无法正常工作**——所有 throw 表达式都会触发 Rust panic 崩溃而非正常的异常传播。根因是异常抛出路径存在双重断裂：

1. **`throw_exception()`**（`ruyi_runtime/src/exception.rs:286`）用 `panic!()` 占位，而非调用真正的 unwinder
2. **C FFI `ruyi_throw`**（`ruyi_runtime/src/c_exports.rs:9`）仅存储 pending exception 指针，不触发栈展开

这导致 codegen 中已具备的完整 try/catch/finally landing pad 基础设施（`stmt.rs:880-958`）和 runtime 中已实现的 `_Unwind_RaiseException` 调用（`exception/runtime.rs:33-49`）都无法被正确激活。

## What Changes

### 1. 连接 throw_exception 到真正的 unwinder

`ruyi_runtime/src/exception.rs:286` 的 `throw_exception()` 当前：
```rust
pub fn throw_exception(exc: RuyiException) -> ! {
    panic!("RuyiException(type_id={}, message={})", exc.type_id, exc.message);
}
```
改为调用 `exception/runtime.rs` 中已存在的 `ruyi_throw(exception: *mut ExceptionObject) -> !`，该函数正确调用 `_Unwind_RaiseException`。

### 2. 升级 C FFI ruyi_throw

`ruyi_runtime/src/c_exports.rs:9` 的 `ruyi_throw(msg: *const i8)` 当前仅存储 pending exception：
```rust
pub extern "C" fn ruyi_throw(msg: *const i8) {
    PENDING_EXCEPTION.store(msg as *mut i8, Ordering::SeqCst);
}
```
改为：从 `msg` 构建 `ExceptionObject`，调用 `throw_exception`（进而触发真正的 unwinder）。

### 3. 同步 ruyic 侧的 Exception::throw

`ruyic/src/runtime/exception.rs:15` 的 `Exception::throw()` 当前也用 `panic!()`。需同步更新使其通过 runtime 路径触发 unwinder。

### 4. 启用 4 个 ignored 测试

`ruyi_runtime/tests/exception_runtime.rs` 中的 4 个 `#[ignore]` 测试在 unwinder 正常工作后应通过：
- `test_ruyi_throw_aborts_when_unwind_returns` (line 45)
- `test_ruyi_end_catch_no_panic_without_active_catch` (line 124)
- `test_function_exception_table_multiple_entries` (line 258)
- `test_nested_try_catch_propagates_to_outer` (line 528)

### 5. 新增端到端 .ry 集成测试

在 `crates/ruyic/tests/integration/cases/codegen/` 中新增：
- `exception_throw.ry`: `throw Error.new("test")` → `catch (e: Error)` 捕获并打印 `e.getMessage()`
- `exception_nested.ry`: 嵌套 try-catch-finally 传播
- `exception_rethrow.ry`: catch 块内 rethrow

## Scope

### In Scope
- `crates/ruyi_runtime/src/exception.rs`: `throw_exception` panic→unwinder
- `crates/ruyi_runtime/src/c_exports.rs`: `ruyi_throw` 升级为真正触发 unwinder
- `crates/ruyic/src/runtime/exception.rs`: `Exception::throw` 同步更新
- `crates/ruyi_runtime/tests/exception_runtime.rs`: 启用 4 个 `#[ignore]` 测试
- `crates/ruyic/tests/integration/cases/codegen/`: 新增 3 个端到端 .ry 测试
- 修复 try_catch_invoke.rs 中因 "Complex new expressions" 限制而跳过的测试（该限制在 Change B 中已解除，此处配合验证）

### Out of Scope
- GC、async、spawn 相关改动
- 新语言特性
- `allow_partial_codegen`、测试断言修复、codegen 12 条路径、路线图更新（Change B 范围）
- Mutex 迁移、CI/CD、tag 补打（优先级 6）
- `compile_throw` 在 codegen 中的"pending exception + return"模式重构（保留现有 return-based 模式作为 unwinder 未就绪时的 fallback）

## Impact

| 模块 | 影响程度 | 风险 |
|------|----------|------|
| Runtime exception | 中 | `_Unwind_RaiseException` 在测试环境可能返回 `_URC_END_OF_STACK`——需处理此情况 |
| C FFI exports | 中 | `ruyi_throw` 行为从"存储指针+返回"变为"不返回"——调用方 codegen 已适配（`compile_throw` 后总是 branch 到 catch/rethrow/unreachable） |
| Codegen throw | 低 | `compile_throw` 已正确处理 throw→catch bb 跳转和 rethrow 路径 |
| 测试 | 低 | 启用 ignored 测试、新增 .ry 集成测试 |

## Capabilities

- **修复**：编译后的 Ruyi 程序中 `throw` 能正常被 `catch` 捕获
- **修复**：嵌套 try-catch-finally 正确传播异常
- **修复**：catch 块内 rethrow 正确工作
- **验证**：4 个 previously-ignored runtime 测试通过
- **验证**：3 个新增端到端 .ry 测试通过

## Success Criteria

1. `cargo test -p ruyi_runtime --test exception_runtime` 全部通过（含 4 个 previously-ignored）
2. `cargo test -p ruyic --test try_catch_invoke` 全部通过
3. 新增 3 个端到端 .ry 集成测试编译运行正确
4. `cargo test -p ruyic --test codegen` 中所有已有通过测试保持通过（零回归）
5. `make check` 通过，零新增 clippy 警告
6. 编译的 Ruyi 程序中 `throw Error.new("msg")` → `catch (e: Error)` 成功捕获并打印 `e.getMessage()`
