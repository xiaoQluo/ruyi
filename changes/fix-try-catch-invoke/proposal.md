# Proposal: 修复 try/catch 代码生成（build_call → build_invoke）

## Why

当前 `compile_try`(stmt.rs:245-378) 在 try 体内对函数调用使用 `build_call`,而 LLVM 异常处理要求 try 体内必须用 `build_invoke` 才能把异常路由到 catch handler。结果是:**被调用函数抛出的异常无法被 catch 捕获**,只能捕获同函数内 `throw` 语句主动抛出的异常。这使得 Ruyi 的 try/catch 实质上**无法用于真实程序**,只对显式 throw 起作用。

TRY_CATCH_AUDIT.md(2026-05-04)已审计并锁定根因,但还未修复。本变更在 `dev/v0.5.5` 分支上落地此修复,作为 v0.5 阶段一标准兑现(`roadmap-zh.md` §阶段一成功标准第 2 条: `try/catch/finally` 端到端工作,含真正的异常传播)。

## What Changes

### 1. compile_try 改用 build_invoke + landing pad

**根因**(已审计): `compile_try` 仅手动分配 `exception_ptr`,对所有函数调用使用 `build_call`,不生成 LLVM `invoke` 指令。

**修复**:
- 把 `compile_try` 改造为生成 LLVM `invoke` 指令(在 try 体内)
- 在 catch block 起始处生成 `landingpad` 指令,匹配异常类型
- catch dispatch 由 `LandingPadGenerator::build_catch_dispatch` 统一处理
- finally block 维持现有合并逻辑,无须改动

### 2. compile_throw 加 unreachable

**根因**(已审计): `compile_throw` 调用 noreturn 函数 `ruyi_throw` 后,没有紧随 `unreachable` 指令,导致后续基本块仍可能流入,生成错误的 PHI 节点。

**修复**:
- 在 `ctx.builder.build_call(throw_fn, ...)` 之后立即 `build_unreachable()`
- try_stack 跳转仍保留,但作为 `unreachable` 之前的引导分支

### 3. expr.rs 函数调用在 try 上下文改用 invoke

**根因**(已审计): `compile_call`(expr.rs:889) 始终 `build_call`,不知道当前是否处于 try 上下文。

**修复**:
- 引入 `CodegenContext.in_try_block: bool`(或 `try_ctx` 栈追踪)状态
- 当 `in_try_block == true`,生成 `invoke` 指令而非 `call`,指定 `landing_bb`
- 函数返回值的合并仍由 caller 处理(catch bb 后追加 PHI 节点)

### 4. LandingPadGenerator 集成

**根因**(已审计): `LandingPadGenerator` 位于 `ruyi_runtime`,藏在 `#[cfg(feature = "inkwell")]`,ruyic 不可访问。

**修复**(架构重构):
- 选项 A(推荐): 新建 `crates/ruyi_exception/src/lib.rs` shared crate,搬入 `LandingPadGenerator` + 异常类型定义,让 `ruyic` 与 `ruyi_runtime` 都依赖
- 选项 B(最小改动): 把 `LandingPadGenerator` 移到 `ruyic/src/codegen/landing_pad.rs`,改为 `ruyic` 独占(不再依赖 `ruyi_runtime`)

本变更采用 **选项 A**,新建 shared crate,理由见 `design.md` §Decisions。

## Scope

### In Scope

- `crates/ruyic/src/codegen/stmt.rs:compile_try` 改造为 invoke + landing pad
- `crates/ruyic/src/codegen/stmt.rs:compile_throw` 加 `unreachable`
- `crates/ruyic/src/codegen/expr.rs:compile_call` 在 try 上下文用 invoke
- `crates/ruyic/src/codegen/generator.rs:CodegenContext` 加 `try_stack` 或 `in_try_block` 状态字段
- **新建** `crates/ruyi_exception/src/lib.rs`(shared crate)
- 把 `crates/ruyi_runtime/src/exception/landing_pad.rs` 内容搬入 shared crate
- 更新 `ruyi_runtime/Cargo.toml` 与 `ruyic/Cargo.toml` workspace 依赖
- 新增 codegen 集成测试 `crates/ruyic/tests/try_catch_invoke.rs`(`#[ignore]`,需 LLVM 环境)
- `examples/try_catch_invoke.ry`: 新增端到端 example(被调用函数抛出 → 外层 catch 捕获)
- `examples/run_examples.sh` 接入新 example
- `docs/spec.md` §7(异常处理):用词微调为 "invoke + landing pad"(如有歧义)

### Out of Scope (Scope Fence)

- ❌ **`ruyi_await` 空操作修复**(独立 P0,见 remaining-issues / roadmap-zh)
- ❌ **GC 老年代标记-压缩**(remaining-issues Task 10)
- ❌ 失败测试 3 项(`test_from_annotation_generic` / `test_bool_patterns_with_wildcard` / `test_check_match_statement`)
- ❌ **`allow_partial_codegen` 全局启用问题**(driver.rs:583)
- ❌ stdlib 模块补全(math/time/json 等)
- ❌ async/await 中的异常传播(unrelated to try/catch invoke 修复)
- ❌ 重新设计 finally 复杂语义(defer 模式、stack unwind 等)
- ❌ catch 类型匹配的多分支优化
- ❌ ruyi_runtime 异常表的 GC 集成优化

## Impact

| 影响面 | 评估 |
|--------|------|
| 编译产物 | try/catch 函数内 IR 由 `call` → `invoke` + `landingpad`,每个 catch 增加 1 个 landing pad block。常见小型程序 +2-5% IR 体积,可忽略 |
| 运行行为 | try 内被调用函数抛出时,异常正确路由到 catch handler(以前会绕过 catch,直接上抛) |
| 测试 | 新增 codegen 集成测试(#[ignore]);examples 数量 33 → 34(34/34 通过) |
| 性能 | 零开销异常已是 spec 承诺,无回归 |
| ABI | 不变(LandingPadGenerator 移到 shared crate 是内部重构,不影响 ABI) |
| 架构 | 新增 shared crate `ruyi_exception`,影响 workspace `Cargo.toml` 的 member 列表 |
| 兼容性 | CI 已在 commit `72fd843` 临时移除 ci.yml(LLVM 14 runner 不可用),本变更 codegen 测试用 `#[ignore]` 保持现状 |

## Capabilities

### 修改能力(MODIFIED)

- `language-exception-handling`: try/catch 实现从手动 try_stack 升级到 LLVM `invoke + landingpad`,正确捕获被调用函数抛出的异常
- `compiler-codegen-stmt`: `compile_try` 与 `compile_throw` 重写
- `compiler-codegen-expr`: `compile_call` 在 try 上下文用 invoke
- `compiler-codegen-context`: `CodegenContext` 新增 try_stack 状态字段
- `runtime-landing-pad`: 从 `ruyi_runtime` 内部模块升级为 shared crate,被 `ruyic` 与 `ruyi_runtime` 共同依赖

### 新增能力(ADDED)

- 无

### 删除能力(REMOVED)

- 无

## Acceptance

```bash
# 1. 编译验证 (无 LLVM 也可)
cargo check --workspace                          → 零警告
cargo check -p ruyi_runtime --no-default-features → 零警告(无 LLVM 环境)
cargo clippy --workspace                          → 零警告

# 2. 测试 (无需 LLVM)
cargo test --workspace                           → 全部通过(回归测试零恶化)
cargo test -p ruyi_runtime --lib --no-default-features → 全部通过

# 3. 端到端 codegen 测试 (需要 LLVM 14)
LLVM_SYS_140_PREFIX=... ruyic examples/try_catch_invoke.ry -o try_catch_invoke
./try_catch_invoke                                → exit 0, 输出含 "caught"
                                                 → 异常从内层被调用函数抛出,被外层 catch 捕获

# 4. examples 套件
bash examples/run_examples.sh                     → Total: 34 | Passed: 34 | Failed: 0

# 5. doc 一致性
TRY_CATCH_AUDIT.md 中的 §5 表格 "Is LandingPadGenerator compatible with codegen?"
                                                  → 答案由 NO 改为 YES
```
