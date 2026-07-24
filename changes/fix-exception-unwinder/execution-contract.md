# 执行合同：fix-exception-unwinder

> **Change**: fix-exception-unwinder | **Branch**: `dev/fix-exception-unwinder`
> **Workflow**: full | **Sequential Batches**: 3 (Runtime → Compiler → Test)

## Intent Lock

- **变更名称**：fix-exception-unwinder
- **要解决的问题**：Ruyi v0.5.9 的 `throw`/`try`/`catch` 在编译后的程序中无法正常工作——所有 throw 触发 Rust panic 崩溃。根因是双重断裂：`throw_exception()` 用 `panic!()` 占位 + C FFI `ruyi_throw` 仅存储 pending pointer，两者均未连接到已实现的 `_Unwind_RaiseException` 路径。
- **范围内**：① `throw_exception` 连接 `exception/runtime.rs:ruyi_throw`（真正的 unwinder）；② C FFI `ruyi_throw` 构建 ExceptionObject 并调用 `throw_exception`；③ ruyic 侧 `Exception::throw` 同步；④ 启用 4 个 `#[ignore]` runtime 测试；⑤ 新增 3 个端到端 .ry 集成测试。
- **范围外**：GC、async、新语言特性、`compile_throw` 重构（保留现有 return-based fallback）、Mutex 迁移、CI/CD（优先级 6）。

## Approved Behavior

### 已批准需求摘要（5 项）

| 编号 | 特性项 | 行为摘要 |
|------|--------|----------|
| R1 | throw_exception→unwinder | `panic!()` 替换为 `ExceptionObject` 构造 + unsafe `ruyi_throw(ptr)` 调用 `_Unwind_RaiseException` |
| R2 | C FFI ruyi_throw 升级 | `*const i8` → CStr → `RuyiException` → `throw_exception`（不再仅存储 pending pointer） |
| R3 | Exception::throw 同步 | ruyic 侧 `panic!()` 移除，与 runtime 对齐 |
| R4 | try/catch 端到端 | 编译后程序 throw→catch 正常捕获；嵌套 try-catch-finally 正确传播；rethrow 正确工作 |
| R5 | 零回归 | `exception_runtime` 全通过（含 4 ignored）；`try_catch_invoke` 12/12；`codegen` 全通过 |

### 关键场景

- **R2** `throw Error.new("test")` 在 try 块内 → landing pad 捕获 → catch 打印 "test"
- **R4 嵌套** 内层 throw → 内层 finally 执行 → 内层 catch 不匹配 → 外层 catch 捕获
- **R4 rethrow** catch 块内 `throw` 新异常 → 外层 try 捕获
- **R1 abort** `_Unwind_RaiseException` 返回 `_URC_END_OF_STACK` → cleanup → `std::process::abort()`

### 验收检查（DP-1 6 项，摘自 proposal.md）

1. `cargo test -p ruyi_runtime --test exception_runtime` 全部通过（含 4 个 previously-ignored，或对无法测试的保留 ignore + 注释原因）
2. `cargo test -p ruyic --test try_catch_invoke` 12/12 通过
3. 新增 3 个端到端 .ry 集成测试（exception_throw / exception_nested / exception_rethrow）编译运行正确
4. `cargo test -p ruyic --test codegen` 零回归
5. `make check` 通过，零新增 clippy 警告
6. 手动验证：编译 `throw Error.new("msg")` → `catch (e: Error)` 打印 `e.getMessage()`（无 Rust panic backtrace）

## Design Constraints

- **架构约束**：
  - 两阶段方案：此 Change 连接 unwinder（验证 landing pad 正确性），后续 Change 优化 `compile_throw` 移除 dead code
  - `throw_exception` 保持 `pub fn` 安全入口，内部 unsafe 边界清晰（`Box::into_raw` → `ruyi_throw(ptr)`）
  - `ruyi_throw` 是 `-> !` 发散函数——编译器识别 `build_call` 后代码为 dead code
  - `PENDING_EXCEPTION` 原子变量保留（可能仍有调用方），标记 deprecated
- **接口约束**：
  1. **exception.rs → runtime.rs**：`throw_exception` 包装 `RuyiException` → `ExceptionObject`，unsafe 调用 `exception::runtime::ruyi_throw(ptr: *mut ExceptionObject) -> !`
  2. **c_exports.rs → exception.rs**：C FFI 构建 `RuyiException`，调用 `throw_exception(exc) -> !`
  3. **Codegen → Runtime**：通过 LLVM `build_call` 调用 `#[no_mangle] ruyi_throw` 符号——符号名不变，行为从 "存储+返回" 变为 "发散"
- **依赖约束**：LLVM 14；Itanium C++ ABI（macOS/Linux 兼容）；Change B Task 3b.3（复杂 new 表达式）完成后，`try_catch_invoke` 中 `throw Error.new(...)` 模式的测试解除阻断

## Task Batches

### Batch 1: Runtime Unwinder Connection（3 tasks，Independent）

| Task | File | Objective |
|------|------|-----------|
| 1.1 | `ruyi_runtime/src/exception.rs:286` | `throw_exception`: `panic!()` → `ExceptionObject::new` + `Box::into_raw` + unsafe `runtime::ruyi_throw(ptr)` |
| 1.2 | `ruyi_runtime/src/c_exports.rs:9` | C FFI `ruyi_throw`: 存指针 → CStr → `RuyiException` → `throw_exception` |
| 1.3 | `ruyi_runtime/src/c_exports.rs` | Audit `PENDING_EXCEPTION` 引用，标记 deprecated / 保留符号 |

### Batch 2: Compiler-Side Throw Update（2 tasks，Independent）

| Task | File | Objective |
|------|------|-----------|
| 2.1 | `ruyic/src/runtime/exception.rs:15` | `Exception::throw`: `panic!()` → 同步 runtime 路径（保留 `std::process::abort()` fallback） |
| 2.2 | `ruyic/src/codegen/stmt.rs:718` | 审查 `compile_throw`：`ruyi_throw` 调用后添加 `build_unreachable()`（告知 LLVM 路径不可达） |

### Batch 3: Test Enablement & Verification（5 tasks，Depends on: Batch 1, Batch 2）

| Task | File(s) | Objective |
|------|---------|-----------|
| 3.1 | `exception_runtime.rs` | 启用 4 个 `#[ignore]` 测试；对需 abort 的用 `#[should_panic]`；对需 LLVM 基础设施的保留 ignore + 注释 |
| 3.2 | `try_catch_invoke.rs` | 运行 12 个测试，修复因 unwinder 行为变化产生的失败；目标 12/12（配合 Change B Task 3b.3） |
| 3.3 | `exception_throw.ry` + `.expected` + `codegen.rs` | 新增端到端测试：basic throw→catch |
| 3.4 | `exception_nested.ry` + `.expected` + `codegen.rs` | 新增端到端测试：嵌套 try-catch-finally 传播 |
| 3.5 | `exception_rethrow.ry` + `.expected` + `codegen.rs` | 新增端到端测试：catch 内 rethrow |

## Test Obligations

- **TDD 核心边界**：每 task 前写测试（或确认当前失败状态）→ 实现 → 验证通过 → 回归
- **Runtime 单元测试**：`cargo test -p ruyi_runtime --lib` — `throw_exception` 相关测试需适配（`#[should_panic]` → 直接验证或保留）
- **Regression 敏感区**：
  - `cargo test -p ruyic --test codegen` — codegen 全量（throw/catch 路径影响）
  - `cargo test -p ruyic --test try_catch_invoke` — 12 个测试
  - `cargo test -p ruyi_runtime --test exception_runtime` — 含 4 个 previously-ignored
- **跨 change 依赖**：Change B Task 3b.3（复杂 new 表达式 codegen）完成后，本 Change Task 3.2（try_catch_invoke）中 `throw Error.new(...)` 模式解除编译阻断

## Review Gates

| Gate | 时机 | 验证内容 |
|------|------|----------|
| G1 | Batch 1 完成 | `cargo check -p ruyi_runtime` 编译通过；`throw_exception` 正确调用 `_Unwind_RaiseException` |
| G2 | Batch 2 完成 | `make check` 通过；`compile_throw` 中 `build_unreachable` 正确安置 |
| G3 | Batch 3.1 完成 | `cargo test -p ruyi_runtime --test exception_runtime` 全通过（或保留 ignore + 注释） |
| G4 | Batch 3.2 完成 | `cargo test -p ruyic --test try_catch_invoke` 12/12 |
| G5 | Batch 3.3-3.5 + FV | `cargo test -p ruyic --test codegen` 全通过（含 3 个新 fixture）+ `cargo test --workspace` 全通过 |

## Escalation Rules

| 层级 | 触发条件 | 动作 |
|------|----------|------|
| 1: Specifying | `_Unwind_RaiseException` 在目标平台行为与预期不符 | 查阅 Itanium C++ ABI 文档，添加平台条件编译（`#[cfg]`）；记录 platform-specific behavior |
| 2: Bridging | `try_catch_invoke` 测试中因 "Complex new expressions" 限制持续失败 | 等待 Change B Task 3b.3 完成后重跑；若 Change B 未完成，标记为 blocked |
| 3: Stop | `cargo test -p ruyic --test codegen` 出现回归（previously-passing 失败） | 停止全部编辑，git stash → 逐文件 diff → 定位变更 → 回滚或修复 |
