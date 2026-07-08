# Execution Contract: fix-try-catch-invoke

**Change**: `fix-try-catch-invoke`
**Mode**: full
**State**: specifying → bridging (DP-2 ✅ approved, pending DP-3 approval)

## Intent Lock

修复 Ruyi 编译器的 `compile_try` 实现,使其在 try 体内对函数调用使用 LLVM `invoke` 指令(而非 `build_call`),使被调用函数抛出的异常能正确路由到外层 catch 块。同时将 `LandingPadGenerator` 从 `ruyi_runtime` 内部模块迁出到新建的 `ruyi_exception` shared crate,供 `ruyic`(编译器)与 `ruyi_runtime`(运行时)共用。最终实现 try/catch 端到端真实异常传播,examples 套件从 33 增至 34。

## Affected Scope

**新建**:
- `crates/ruyi_exception/Cargo.toml` — shared crate 清单
- `crates/ruyi_exception/src/lib.rs` — crate root
- `crates/ruyi_exception/src/landing_pad.rs` — 搬迁自 ruyi_runtime 的 LandingPadGenerator
- `crates/ruyic/tests/try_catch_invoke.rs` — codegen 集成测试(`#[ignore]`,需 LLVM 14)
- `examples/try_catch_invoke.ry` — 端到端 example

**修改**:
- `Cargo.toml` (workspace root) — 添加 ruyi_exception 到 members
- `crates/ruyi_runtime/src/exception/landing_pad.rs` — 替换为 re-export
- `crates/ruyi_runtime/Cargo.toml` — 添加 ruyi_exception 依赖
- `crates/ruyic/Cargo.toml` — 添加 ruyi_exception 依赖(启用 `llvm14` feature)
- `crates/ruyic/src/codegen/generator.rs` — CodegenContext 新增 `try_stack: Vec<TryFrame>` 与 RAII guard
- `crates/ruyic/src/codegen/stmt.rs` — `compile_try` 改写 + `compile_throw` 加 unreachable
- `crates/ruyic/src/codegen/expr.rs` — `compile_call` 在 try 内改用 invoke
- `examples/run_examples.sh` — 接入 try_catch_invoke,总数 33 → 34
- `TRY_CATCH_AUDIT.md` — §3 末尾注释 + §5 表格三行更新

## Requirement Coverage (Cross-Check)

| Requirement | Source | Mapped Batch | Test Obligation |
|------------|--------|--------------|------------------|
| REQ-TCI-001 catch exception from called functions | specs/01 | T4 + T5 + T7 | examples/try_catch_invoke.ry 通过编译并打印 `caught`; LLVM IR 含 invoke |
| REQ-TCI-002 throw unreachable | specs/01 | T3 + T7 | throwStmt 后 IR 含 unreachable;unit test 验证 |
| REQ-TCI-003 compile_call respects try context | specs/01 | T5 + T7 | codegen test: try 内 call → invoke, try 外 call → call |
| REQ-TCI-004 CodegenContext try_stack tracking | specs/01 | T2 + T4 | unit test: push/pop 平衡;嵌套 try 后 try_stack 为空 |
| REQ-TCI-005 LLVM IR landingpad + dispatch | specs/01 | T4 | codegen test: catch 块含 landingpad |
| REQ-TCI-006 codegen integration test | specs/01 | T7 | `cargo test -p ruyic --test try_catch_invoke -- --ignored` 通过 |
| REQ-TCI-007 examples/try_catch_invoke.ry | specs/01 | T6 + T7 | `bash examples/run_examples.sh` → 34/34 |
| REQ-LPG-001 workspace member ruyi_exception | specs/02 | T1 | `cargo check --workspace` 通过 |
| REQ-LPG-002 ruyi_exception 公开 LandingPadGenerator | specs/02 | T1 | `cargo check -p ruyic` 通过;ruyi_runtime 仍可引用 |
| REQ-LPG-003 remove from ruyi_runtime::exception | specs/02 | T1 | `cargo check -p ruyi_runtime --no-default-features` 通过 |
| REQ-LPG-004 reorganize workspace deps | specs/02 | T1 | `cargo tree -p ruyic` 含 ruyi_exception |
| REQ-LPG-005 update TRY_CATCH_AUDIT.md | specs/02 | T8 | §5 表格三行答案由 NO 改为 YES |

**Coverage: 12/12 Requirements mapped** — no unmapped requirements.

## Task Batches

### Batch 1: 基础设施(2 项并行)
- **目标**: 落地 `ruyi_exception` shared crate 与 `try_stack` 状态字段
- **输入**: ruyi_runtime::exception::landing_pad 现有实现
- **输出**: ruyi_exception crate 可被 ruyic 与 ruyi_runtime 引用;CodegenContext.try_stack 可用
- **完成标准**:
  - `cargo check --workspace` 通过
  - `cargo check -p ruyi_runtime --no-default-features` 通过
  - `cargo check -p ruyi_exception --features llvm14` 通过
  - `test_try_stack_push_pop` 单元测试通过

| ID | Action | Files | LOC | Verify |
|----|--------|-------|-----|--------|
| T1 | 搬迁 LandingPadGenerator 到 ruyi_exception shared crate | `crates/ruyi_exception/{Cargo.toml,src/lib.rs,src/landing_pad.rs}`, `crates/ruyi_runtime/src/exception/landing_pad.rs`, `crates/ruyic/Cargo.toml`, `crates/ruyi_runtime/Cargo.toml`, `Cargo.toml`(workspace) | ~80 | workspace check 通过;ruyi_runtime no-default-features 通过 |
| T2 | CodegenContext 新增 `try_stack: Vec<TryFrame>` 与 RAII guard `TryStackGuard` | `crates/ruyic/src/codegen/generator.rs` | ~50 | `test_try_stack_push_pop` 通过 |

### Batch 2: 核心改造(2 项并行,依赖 Batch 1)
- **目标**: 改造 `compile_throw` 与 `compile_try`, 引入 invoke + landingpad
- **输入**: Batch 1 提供的 try_stack + LandingPadGenerator
- **输出**: try 体函数调用改用 invoke;catch 块含 landingpad;throw 末尾 unreachable
- **完成标准**:
  - `cargo check -p ruyic` 通过
  - `cargo test -p ruyic --lib` 通过(无回归)
  - `examples/try_catch_invoke.ry` 临时 inline 验证通过

| ID | Action | Files | LOC | Verify |
|----|--------|-------|-----|--------|
| T3 | `compile_throw` 末尾添加 unreachable | `crates/ruyic/src/codegen/stmt.rs` | ~15 | IR 含 `unreachable` 指令 |
| T4 | `compile_try` 改用 build_invoke + landingpad + LandingPadGenerator dispatch | `crates/ruyic/src/codegen/stmt.rs` | ~120 | 端到端 try/catch 验证通过;IR 含 `invoke + landingpad` |

### Batch 3: 调用方改造(1 项,依赖 Batch 2)
- **目标**: `compile_call` 感知 try 上下文, 生成 invoke 或 call
- **输入**: Batch 2 提供的 `try_stack` 字段
- **输出**: try 内函数调用一律 invoke;try 外调用保持 call
- **完成标准**:
  - `cargo test -p ruyic --lib` 通过(无回归)
  - `cargo clippy -p ruyic` 零警告

| ID | Action | Files | LOC | Verify |
|----|--------|-------|-----|--------|
| T5 | `compile_call` 在 try_stack 非空时生成 `build_invoke` | `crates/ruyic/src/codegen/expr.rs` | ~40 | codegen test: try 内 invoke, try 外 call |

### Batch 4: 验证、新 example 与文档(3 项并行)
- **目标**: 端到端 example, codegen 集成测试,文档更新
- **输入**: Batch 1-3 完成项
- **输出**: examples 33 → 34; codegen test #[ignore] 即可; TRY_CATCH_AUDIT.md 更新
- **完成标准**:
  - `bash examples/run_examples.sh` → Total: 34 | Passed: 34 | Failed: 0(需 LLVM 14)
  - `cargo test -p ruyic --test try_catch_invoke -- --ignored` 通过(LLVM)
  - TRY_CATCH_AUDIT.md §5 表格三行答案更新

| ID | Action | Files | LOC | Verify |
|----|--------|-------|-----|--------|
| T6 | 新增 `examples/try_catch_invoke.ry` 并接入 run_examples.sh | `examples/try_catch_invoke.ry`, `examples/run_examples.sh` | ~30 | example 编译并打印 `caught` |
| T7 | 新增 codegen 集成测试(全部 #[ignore]) | `crates/ruyic/tests/try_catch_invoke.rs` | ~50 | `cargo test -p ruyic --test try_catch_invoke -- --ignored` 通过 |
| T8 | 更新 TRY_CATCH_AUDIT.md §3 + §5 | `TRY_CATCH_AUDIT.md` | ~10 | §5 表格三行答案改为 YES/YES/YES |

## Test Obligations

### 必须从失败测试开始的行为(TDD-RED 起点)
- T2: `test_try_stack_push_pop` (验证 push/pop 平衡)
- T3: codegen test 验证 throw 后 IR 含 unreachable
- T5: codegen test 验证 try 内 invoke, try 外 call
- T7: codegen test 验证 IR 含 landingpad + invoke

### 必需的边界情况
- 嵌套 try(2 层):内层 catch 是否接住内层 try 体异常;外层 catch 接住外层
- 多个 catch arm with selector:ErrorA 抛 → 第一个 catch 命中
- finally 在正常与异常路径都执行
- 没有 catch 的 try(只有 finally):异常仍经 finally 后上抛
- 函数带返回值在 try 内:invoke 后的 PHI 节点正确
- 嵌套 try 退出后,后续 call 仍用 call(确认 try_stack pop 正确)

### 回归敏感区域
- `examples/io.ry`、`examples/error.ry`(已有 try/catch)行为不变
- 现有 33 个 example 全通过(33 → 34)
- `cargo test --workspace` 无新增失败
- `cargo test -p ruyi_runtime --no-default-features --lib` 全通过

## Design Constraints

### 架构约束(来自 D1-D7)
- **D1**: `compile_try` 改用 `build_invoke`,landingpad + dispatch 由 `LandingPadGenerator` 统一处理
- **D2**: `LandingPadGenerator` 必须位于 `ruyi_exception` shared crate;**禁止**把 ruyic 重写或反向依赖 ruyi_runtime
- **D3**: `CodegenContext.try_stack: Vec<TryFrame>` 栈式结构(嵌套支持);**禁止**单 boolean
- **D4**: `compile_call` 对 try 内**所有** call 生成 invoke;**禁止**仅对 `throw` 标注的函数生成 invoke
- **D5**: `compile_throw` 跳转分支后必须 `unreachable`;保留现有 try_stack 跳转
- **D6**: codegen 集成测试用 `#[ignore]`;**禁止**强行 CI 启用(LLVM runner 不可用)
- **D7**: 不重写整个 LandingPadGenerator;仅适配 TypeId→TryTypeId

### 接口约束
- `ruyi_exception::landing_pad::LandingPadGenerator<'ctx, 'm, 'b>` 签名保持兼容
- `TryTypeId = u32` 替换原 `TypeId`(整数 type id 解耦)
- `TryFrame { landing_pad_bb, catch_bb, finally_bb, exception_ptr }` 嵌套 try 上下文
- `TryStackGuard<'a>` RAII 守卫,Drop 时自动 pop

### 依赖约束
- 新增 workspace member `crates/ruyi_exception`
- `ruyic` 启用 `ruyi_exception` 的 `llvm14` feature(强依赖 inkwell)
- `ruyi_runtime` 依赖 `ruyi_exception` 但**不**启用 `llvm14` feature(走 opaque 调用)
- `default-features = false` 保证 `--no-default-features` 路径仍可编译

### 数据约束
- `try_stack` 在 `CodegenContext` 中是字段,非全局状态(避免多线程冲突,Ruyi 编译器目前非并行)
- `LandingPadGenerator` 实例生命周期 `'a` ≤ `'ctx`/`'m`/`'b`(inkwell lifetime 约束)

## Out of Scope (Scope Fence)

- ❌ `ruyi_await` 空操作修复(独立 P0,跨 async module)
- ❌ 老年代 GC 标记-压缩(remaining-issues Task 10)
- ❌ 3 个失败测试(`test_from_annotation_generic` / `test_bool_patterns_with_wildcard` / `test_check_match_statement`)
- ❌ `allow_partial_codegen` 全局启用(driver.rs:583)
- ❌ stdlib 模块补全(math/time/json/regex/random/fmt/net/buffer/test)
- ❌ async/await 中的异常传播
- ❌ finally 复杂语义(defer 模式、stack unwinding 等)
- ❌ catch 类型匹配的多分支优化
- ❌ ruyi_runtime 异常表 GC 集成
- ❌ CI 修复 LLVM 14 runner(独立工作,本变更不动 ci.yml)
- ❌ 已有 example 行为的优化或重构

## Execution Mode

- **模式**: `Batch Inline`
- **选择理由**:
  - 工作量 ~400-500 行代码改动,跨 5 个 crate(workspace + ruyic + ruyi_runtime + ruyi_exception 新建 + examples)
  - 包含架构变更(新增 shared crate),需谨慎分阶段验证
  - 4 个 Batch 中 Batch 1 / Batch 2 / Batch 4 内部可并行
  - Batch 之间有强依赖(Batch 2 依赖 Batch 1, Batch 3 依赖 Batch 2)
  - 不适合纯 Inline 单 agent 串行执行,适合分批委派 Sisyphus 子 agent 并行+有状态回顾

## Verification Dimensions

| 维度 | 状态 | 发现 |
|------|------|------|
| Completeness | Pending | 12/12 Requirements mapped;4 batches,9 atomic tasks |
| Correctness | Pending | T7 codegen tests 验证 LLVM IR(含 invoke + landingpad + unreachable) |
| Coherence | Pending | design.md 7 decisions 与 tasks.md 4 waves 一致;跨 batch 依赖明确 |

**总体结论**: Pending — 待 DP-3 批准 + 实施完成后回填

## Review Gates

- **强制审查点**: 每个 Batch 完成后进入下一 Batch 前,必须:
  1. 运行 `cargo check --workspace`,零警告
  2. 运行 `cargo test --workspace`,无新增失败
  3. 验证本 Batch 完成标准
- **阻塞类别**:
  - 编译错误(`cargo check` 失败)
  - 任何新引入的测试失败
  - IR 中缺少 `invoke`/`landingpad`/`unreachable` 的关键指令
  - 新引入的 `unwrap()`、`as any`、`@ts-ignore` 等反模式

## Escalation Rules

- **何时回退到 `specifying`**: 任一 Batch 的 Acceptance Criteria 失败 3 次或以上,scope 出现未预计的新需求,或 contract 与任务实现实际偏离超过 20% 时
- **何时回退到 `bridging`**: 单 Batch 内发现新的技术债需要在 contract 中追加 Requirements 时
- **何时不得继续实现**:
  - LLVM 14 环境不可用 且 codegen tests 不能运行时(暂停 T7,其余可继续)
  - 现有 example 回归超过 5% 时(暂停,回退到 `specifying`)
  - `compile_throw`/try/catch 行为破坏现有 program test path 时

## Handoff Rules

- Batch 1 → Batch 2 → Batch 3 → Batch 4 顺序执行
- Batch 1 内部 T1 / T2 可并行
- Batch 2 内部 T3 / T4 可并行
- Batch 4 内部 T6 / T7 / T8 可并行
- 任一 Batch 失败:停下,记录失败证据,回退到 `specifying` 重新评估

## Ambiguity Flags (Resolved)

- ✅ `LandingPadGenerator` 是否要重写? 已确认 **不要**,仅迁移
- ✅ `try_stack` 用栈还是单 boolean? 已确认 **栈**(嵌套支持需要)
- ✅ 异常处理是否对 async fn 生效? 已确认 **不在本变更范围**(REQ-LPG 不涉及)
- ✅ `compile_throw` 是否改动 try_stack 跳转逻辑? 已确认 **保留**,仅追加 unreachable
- ✅ `examples/run_examples.sh` 接入方式? 已确认 **直接添加 try_catch_invoke 一行**(总数 33 → 34)
- ✅ codegen tests 是否进 CI? 已确认 **不**,CI 已被 commit `72fd843` 临时移除 ci.yml

## Approval Gate (DP-3)

需用户明确批准后进入 `approved-for-build` 状态。批准后:

```bash
ssf state set changes/fix-try-catch-invoke dp_3_result "approved: 4 batches / 9 tasks / 12 mapped requirements / Scope fence 11 items"
ssf state set changes/fix-try-catch-invoke dp_3_timestamp $(date -u +%Y-%m-%dT%H:%M:%SZ)
```

---

**请确认 (DP-3)**:以上契约是否符合预期?批准后立即进入执行阶段 (`approved-for-build`)。
