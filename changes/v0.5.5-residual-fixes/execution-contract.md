# Execution Contract: v0.5.5-residual-fixes

**Change**: `v0.5.5-residual-fixes`
**Mode**: full
**State**: specifying → bridging (DP-2 ✅ approved, pending DP-3 approval)
**Branch**: `dev/v0.5.5`

## Intent Lock

修复 Ruyi v0.5.5 中 **7 项 P0 缺陷**，分 4 阶段交付：(1) GC 双模式 + 静态链接 + T9 收尾 + trait 约束检查（4 项并行）；(2) `ruyi_await` 真实异步 + try/catch landing pad（2 项并行）；(3) `spawn` 内建；(4) 整体回归与归档。最终实现：21+5+13+3+spawn = **至少 42 个** `#[ignore]` 测试启用、async 真正运行、单文件可执行二进制、33 → 34 examples（新增 spawn_demo）、P1+ 缺陷明确划归后续 change。

## Affected Scope

**新建**（14 文件）:

| 路径 | 职责 |
|------|------|
| `crates/ruyic/src/cli/gc_mode.rs` | `--gc=<mode>` 标志解析 |
| `crates/ruyic/src/codegen/gc_alloc.rs` | GC 调用分发（stub/real） |
| `crates/ruyic/src/codegen/builtins/spawn.rs` | `spawn` 内建 LLVM IR 生成 |
| `crates/ruyic/src/typechecker/impl_table.rs` | trait impl HashMap 表 |
| `crates/ruyi_runtime/src/sched/mod.rs` | 调度器主模块 |
| `crates/ruyi_runtime/src/sched/worker.rs` | 工作窃取 worker |
| `crates/ruyi_runtime/src/sched/injector.rs` | 调度入口 |
| `crates/ruyi_runtime/src/async_runtime.rs` | `ruyi_await` 真实实现 |
| `crates/ruyic/tests/gc_flag.rs` | GC flag 集成测试 |
| `crates/ruyic/tests/trait_bounds.rs` | trait 约束检查测试 |
| `crates/ruyi_runtime/tests/spawn.rs` | spawn 集成测试 |
| `examples/spawn_demo.ry` | spawn 演示（3+ 并发任务） |
| `examples/async_sleep.ry` | async/await 演示 |
| `examples/trait_bounds.ry` | trait 约束演示 |
| `tools/audit-stdlib/src/main.rs` | stdlib 8 模块审计工具 |

**修改**（18 文件）:

| 路径 | 修改 |
|------|------|
| `crates/ruyic/src/main.rs` | 新增 `--gc=<mode>` CLI flag |
| `crates/ruyic/src/driver.rs` | `--gc=real` 时链入 `libruyi_runtime.a` |
| `crates/ruyic/src/codegen/mod.rs` | 集成 `gc_alloc.rs` 分发 |
| `crates/ruyic/src/codegen/stmt.rs` | `compile_try` 完整 invoke + landing pad |
| `crates/ruyic/src/codegen/expr.rs` | `compile_call` 在 try 上下文用 invoke |
| `crates/ruyic/src/codegen/generator.rs` | `CodegenContext.try_stack` 状态字段 |
| `crates/ruyic/src/codegen/async_codegen.rs` | `ruyi_await` 真实化 |
| `crates/ruyic/src/codegen/builtins/mod.rs` | 注册 `spawn` 内建 |
| `crates/ruyic/src/typechecker/generics.rs` | `check_bounds` 实际验证 |
| `crates/ruyic/src/typechecker/mod.rs` | 集成 `impl_table.rs` |
| `crates/ruyic/tests/codegen.rs` | 移除 21 个 `#[ignore]` |
| `crates/ruyic/tests/typechecker.rs` | 移除至少 5 个 `#[ignore]` |
| `crates/ruyic/tests/try_catch_invoke.rs` | 移除 13 个 `#[ignore]` |
| `crates/ruyic/tests/compilation_throw_unreachable.rs` | 移除 3 个 `#[ignore]` |
| `stdlib/collections.ry` | `RangeError` / `ArrayIterator` 构造器化 |
| `crates/ruyic/Cargo.toml` | 新增 `crossbeam-deque` |
| `crates/ruyi_runtime/Cargo.toml` | 新增 `crossbeam-deque` |
| `examples/run_examples.sh` | 接入 4 个新 example（33 → 34） |
| `docs/roadmap-zh.md` | P0 缺陷表更新为 ✅ |
| `docs/stdlib-audit-v0.5.5.md` | 审计报告输出（由 audit-stdlib 工具生成） |

## Requirement Coverage (Cross-Check)

| Requirement | Source | Mapped Batch | Test Obligation |
|------------|--------|--------------|-----------------|
| REQ-COLL-001 RangeError constructible | specs/01 | Batch 1.3 (T-1.3.1) | `throw RangeError(...)` 编译通过；codegen.rs 测试 |
| REQ-COLL-002 ArrayIterator constructible | specs/01 | Batch 1.3 (T-1.3.1) | `ArrayIterator(arr)` 编译通过；iterator.next() 运行 |
| REQ-COLL-003 21 codegen tests PASS | specs/01 | Batch 1.3 (T-1.3.2) | `cargo test --test codegen -- --ignored --test-threads=1` ≥21 PASS |
| REQ-COLL-004 stdlib 8 模块审计 | specs/01 | Batch 1.3 (T-1.3.3) | `tools/audit-stdlib` 产出报告含 8 模块 |
| REQ-GC-001 `--gc` flag 接受 | specs/02 | Batch 1.1 (T-1.1.1, T-1.1.3) | stub 默认；real 启用；非法值报错 |
| REQ-GC-002 `--gc=real` 触发静态链接 | specs/02 | Batch 1.1 (T-1.1.4) + Batch 1.2 (T-1.2.2) | `ldd` 无 ruyi_runtime 动态引用 |
| REQ-GC-003 stub allocator 保留 | specs/02 | Batch 1.1 (T-1.1.4) | 33 examples 在 stub 模式行为不变 |
| REQ-LINK-001 driver 链入 `.a` | specs/03 | Batch 1.2 (T-1.2.2) | `ldd ./hello | grep ruyi_runtime` 无输出 |
| REQ-LINK-002 ruyi_runtime 产 `.a` | specs/03 | Batch 1.2 (T-1.2.1) | `target/release/libruyi_runtime.a` 存在 |
| REQ-LINK-003 33 examples 不退化 | specs/03 | Batch 1.2 (T-1.2.2) | run_examples.sh 33/33 PASS |
| REQ-TRAIT-001 check_bounds 实际验证 | specs/04 | Batch 1.4 (T-1.4.2) | `fn f<T: A>(x: T) {} f(42)` 缺 impl 时编译报错 |
| REQ-TRAIT-002 5+ typechecker tests PASS | specs/04 | Batch 1.4 (T-1.4.3) | `cargo test --test typechecker` ≥5 启用并 PASS |
| REQ-TRAIT-003 独立 impl 块支持 | specs/04 | Batch 1.4 (T-1.4.1) | `impl Printable for int` 类外编译通过 |
| REQ-AWAIT-001 ruyi_await 真异步 | specs/05 | Batch 2.1 (T-2.1.2, T-2.1.3) | ready 立即返回；pending 挂起并恢复 |
| REQ-AWAIT-002 工作窃取调度 | specs/05 | Batch 2.1 (T-2.1.1) | 8 worker 注入 100 任务全部完成且负载均衡 |
| REQ-AWAIT-003 async 示例运行 | specs/05 | Batch 2.1 (T-2.1.3) | `examples/async_sleep.ry` 输出 before + after |
| REQ-LPAD-001 invoke 指令 | specs/06 | Batch 2.2 (T-2.2.2) | 跨函数 try/catch 编译并输出 `caught` |
| REQ-LPAD-002 landingpad dispatch | specs/06 | Batch 2.2 (T-2.2.2) | 多 catch arm 正确路由 |
| REQ-LPAD-003 13 try_catch tests PASS | specs/06 | Batch 2.2 (T-2.2.3) | `cargo test --test try_catch_invoke -- --ignored` ≥13 PASS |
| REQ-LPAD-004 3 throw_unreachable tests PASS | specs/06 | Batch 2.2 (T-2.2.3) | `cargo test --test compilation_throw_unreachable -- --ignored` ≥3 PASS |
| REQ-SPAWN-001 spawn 内建 | specs/07 | Batch 3 (T-3.1) | `spawn(fn)` 编译运行 |
| REQ-SPAWN-002 spawn 仅 real 模式 | specs/07 | Batch 3 (T-3.1) | stub 模式 `spawn(...)` 编译报错 |
| REQ-SPAWN-003 spawn_demo 示例 | specs/07 | Batch 3 (T-3.2) | `bash examples/run_examples.sh` 34/34 PASS |

**Coverage: 23/23 Requirements mapped** — no unmapped requirements.

## Task Batches

### Batch 1.1: GC 双模式（4 任务，依赖无）

**目标**: `--gc=stub` / `--gc=real` 编译标志贯穿 CLI → driver → codegen
**完成标准**:
- `cargo test -p ruyic gc_mode` + `gc_alloc` + `gc_flag` 全绿
- `cargo clippy -p ruyic` 零警告
- 33 examples 在 stub 模式行为不变

| ID | 任务 | 依赖 |
|----|------|------|
| T-1.1.1 | GcMode enum + parse | — |
| T-1.1.2 | GcAllocFn codegen 分发 | T-1.1.1 |
| T-1.1.3 | CLI `--gc` flag 接入 | T-1.1.1 |
| T-1.1.4 | codegen 全部堆分配点切换 | T-1.1.2, T-1.1.3 |

### Batch 1.2: 静态链接（2 任务，依赖 Batch 1.1 T-1.1.3）

**目标**: `cargo build -p ruyi_runtime --release` 产 `libruyi_runtime.a`，driver 在 `--gc=real` 下链入
**完成标准**:
- `target/release/libruyi_runtime.a` 存在且非空
- `ldd ./hello` 无 ruyi_runtime 动态引用
- 33 examples 仍 PASS

| ID | 任务 | 依赖 |
|----|------|------|
| T-1.2.1 | ruyi_runtime crate-type 含 staticlib | — |
| T-1.2.2 | driver 链入 `.a`（`--gc=real` 触发） | T-1.2.1, T-1.1.3 |

### Batch 1.3: T9 收尾 + stdlib 审查（3 任务，依赖 Batch 1.1 T-1.1.4）

**目标**: `RangeError` / `ArrayIterator` 构造器化；21 个 codegen 测试启用；stdlib 8 模块审计报告产出
**完成标准**:
- `cargo test --test codegen -- --ignored --test-threads=1` ≥21 PASS
- `tools/audit-stdlib` 输出含 8 模块评估
- `docs/stdlib-audit-v0.5.5.md` 落卷

| ID | 任务 | 依赖 |
|----|------|------|
| T-1.3.1 | RangeError / ArrayIterator 构造器补全 | T-1.1.4 |
| T-1.3.2 | 启用 21 个 codegen `#[ignore]` | T-1.3.1 |
| T-1.3.3 | stdlib audit 工具 + 报告 | T-1.3.1 |

### Batch 1.4: trait 约束检查（3 任务，依赖无）

**目标**: `check_bounds` 实际验证 impl 存在；至少 5 个 typechecker 测试 PASS
**完成标准**:
- `cargo test -p ruyic impl_table` 全绿
- `cargo test -p ruyic --test typechecker` ≥5 启用并 PASS
- 现有泛型 examples 不退化

| ID | 任务 | 依赖 |
|----|------|------|
| T-1.4.1 | ImplTable 数据结构 | — |
| T-1.4.2 | check_bounds 实际验证 | T-1.4.1 |
| T-1.4.3 | 启用 5+ typechecker `#[ignore]` | T-1.4.2 |

### Batch 2.1: ruyi_await 真实化（3 任务，依赖无）

**目标**: 工作窃取调度器 + `ruyi_await` 真实化 + codegen 调用
**完成标准**:
- `cargo test -p ruyi_runtime sched` 全绿（含 work-stealing 测试）
- `loom` 并发测试无 data race
- `examples/async_sleep.ry` 编译运行输出 before + after

| ID | 任务 | 依赖 |
|----|------|------|
| T-2.1.1 | Scheduler + Worker + Injector | — |
| T-2.1.2 | ruyi_await 真实实现 | T-2.1.1 |
| T-2.1.3 | codegen 调用 ruyi_await | T-2.1.2 |

### Batch 2.2: try/catch landing pad（3 任务，依赖 Batch 2.1 + Batch 1.2）

**目标**: `compile_try` 完整 invoke + landing pad，跨函数异常路由；13+3 个测试 PASS
**完成标准**:
- 跨函数 `try { innerThrow(); } catch (e) {}` 编译运行输出 `caught`
- `cargo test --test try_catch_invoke -- --ignored` ≥13 PASS
- `cargo test --test compilation_throw_unreachable -- --ignored` ≥3 PASS

| ID | 任务 | 依赖 |
|----|------|------|
| T-2.2.1 | CodegenContext.try_stack | — |
| T-2.2.2 | compile_try 完整 invoke | T-2.2.1, T-1.2.2 |
| T-2.2.3 | 启用 16 个 try/catch `#[ignore]` | T-2.2.2 |

### Batch 3: spawn 内建（3 任务，依赖 Batch 2.1）

**目标**: `spawn(fn)` 内建 + 示例 + 集成测试
**完成标准**:
- `spawn(fn)` 编译运行；stub 模式编译报错
- `examples/spawn_demo.ry` 编译运行输出 3 个 task 标识
- `cargo test -p ruyi_runtime --test spawn` 全绿

| ID | 任务 | 依赖 |
|----|------|------|
| T-3.1 | spawn builtin IR 生成 | T-2.1.1 |
| T-3.2 | spawn_demo example | T-3.1 |
| T-3.3 | spawn 集成测试 | T-3.1 |

### Batch 4: 验证与归档（2 任务，依赖 Batch 1-3 全部）

**目标**: 整体回归 + release-archivist
**完成标准**:
- `cargo test --workspace` 全绿（除合理保留 `#[ignore]`）
- `cargo clippy --workspace` 零警告
- `bash examples/run_examples.sh` 34/34 PASS
- `docs/roadmap-zh.md` P0 表所有 ✅

| ID | 任务 | 依赖 |
|----|------|------|
| T-4.1 | 整体回归 + roadmap 更新 | Batch 1-3 |
| T-4.2 | release-archivist 流程 | T-4.1 |

## Test Obligations

### 必须从失败测试开始（TDD-RED 起点）

- T-1.1.1: `gc_mode::tests::parse_stub_returns_stub`
- T-1.1.4: `gc_flag.rs::default_mode_uses_cc_alloc`
- T-1.2.2: `driver::tests::real_mode_links_static`
- T-1.3.2: `codegen.rs::range_error_throws_compiles`
- T-1.4.2: `typechecker.rs::generic_with_no_impl_fails`
- T-2.1.1: `sched::tests::submit_one_task_runs_it`
- T-2.1.2: `async_runtime::tests::await_ready_future_returns_immediately`
- T-2.2.2: `try_catch_invoke.rs::inner_throw_caught_by_outer`
- T-3.1: `builtins::spawn::tests::spawn_emits_call_to_scheduler_spawn`

### 必需边界情况

- **GC**: stub/real 切换时 33 examples 行为一致
- **链接**: 单文件二进制（macOS `otool -L` / Linux `ldd` 验证无 ruyi_runtime 动态依赖）
- **T9**: `RangeError` 多 catch arm 路由正确
- **trait**: 多 bound `T: A + B` 同时验证
- **await**: pending future 跨 worker 调度
- **landing pad**: 多 catch arm + selector dispatch
- **spawn**: stub 模式编译报错；real 模式 fire-and-forget

### 回归敏感区域

- 现有 33 examples 全 PASS（含 error.ry、io.ry 已有 try/catch）
- `cargo test --workspace` 无新增 FAIL
- `cargo test -p ruyi_runtime --no-default-features --lib` 全 PASS
- `cargo clippy --workspace` 零警告

## Design Constraints

### 架构约束（来自 D1-D8）

- **D1**: GC 双模式用编译时 flag 切换，**禁止**运行时切换（增加二进制体积）
- **D2**: 静态链接用 `cc-rs` crate，**禁止**直接 `Command::new("cc")`
- **D3**: `RangeError` / `ArrayIterator` 走通用类构造函数路径，**禁止**白名单特殊处理
- **D4**: trait 约束检查用 `HashMap<(TraitId, TypeId), ImplDef>`，**禁止** AST 全遍历
- **D5**: ruyi_await 用 stackless coroutine，**禁止** stackful（与 LLVM 定位不符）
- **D6**: 工作窃取调度器用 `crossbeam-deque`，**禁止** 手写无锁队列
- **D7**: spawn fire-and-forget，**禁止** JoinHandle 类型（推迟后续 change）
- **D8**: stdlib 审查输出报告，**禁止** 实装 math/time/json

### 接口约束

| 接口 | 约束 |
|------|------|
| `GcMode::parse(&str) -> Result<Self, String>` | 接受 `"stub"` / `"real"`，其他 Err |
| `GcAllocFn::for_mode(GcMode) -> Self` | stub→cc_alloc；real→ruyi_gc_alloc |
| `ImplTable::has_impl(TraitId, TypeId) -> bool` | O(1) 查询 |
| `Scheduler::spawn(Future)` | 提交即返回 |
| `Scheduler::yield_now()` | 当前协程挂起 |
| `compile_spawn` | stub 模式拒绝 |

### 依赖约束

- 新增 `crossbeam-deque = "0.8"` 到 `ruyic/Cargo.toml` 与 `ruyi_runtime/Cargo.toml`
- `--gc=real` 需 LLVM 14（`LLVM_SYS_140_PREFIX` 必须设置）
- `--gc=stub` 无 LLVM 依赖（保持现有 `cargo check -p ruyic` 路径）
- `cargo check -p ruyi_runtime --no-default-features` 必须 PASS

### 数据约束

- `try_stack` 是 `CodegenContext` 字段（**非**全局变量）—— 避免多线程冲突
- `Scheduler.workers` 是 `Vec<Worker>`，worker 数 = `num_cpus::get()`
- `ImplTable` 是全局单例（在 typechecker 模块内）

## Out of Scope (Scope Fence)

- ❌ P1/P2/P3 缺陷（12+ 项）—— 后续 change
- ❌ stdlib/math.ry, stdlib/time.ry, stdlib/json.ry —— 后续 change
- ❌ 性能优化、二进制压缩
- ❌ 文档/tutorial 大幅重写（除 roadmap P0 表）
- ❌ 失败测试 3 项历史遗留（`test_from_annotation_generic` 等）
- ❌ finally 复杂语义（defer、stack unwind）
- ❌ catch 类型匹配的多分支优化
- ❌ ruyi_runtime 异常表的 GC 集成优化
- ❌ v0.2-codegen-gaps 历史 tasks.md（归档至 `docs/archive/`）
- ❌ stdlib合理性检查 独立 change（已并入本 change Batch 1.3 T-1.3.3）
- ❌ fix-try-catch-invoke 进一步修复（已 closing，阶段 2 启动前先归档）
- ❌ CI 修复 LLVM runner
- ❌ 已有 example 行为的优化或重构

## Execution Mode

- **模式**: `Batch Inline (SDD pattern: dispatched subagents per task)`
- **选择理由**:
  - 工作量 ~1500-2000 行代码改动，跨 5+ crate
  - 包含架构变更（GC flag、scheduler、trait table、spawn builtin），需谨慎分阶段
  - 阶段 1 内 4 个 batch 并行（GC / 链接 / T9 / trait），阶段 2 内 2 个 batch 并行（await / landing pad）
  - 阶段之间有强依赖（阶段 2 依赖阶段 1，阶段 3 依赖阶段 2）
  - 适合分批委派 Sisyphus-Junior 子 agent 并行 + 有状态回顾

## Verification Dimensions

| 维度 | 状态 | 发现 |
|------|------|------|
| Completeness | ✅ Confirmed | 23/23 Requirements mapped；8 batches, 23 atomic tasks |
| Correctness | Pending | T-1.3.2/T-1.4.3/T-2.2.3/T-3.3 等 codegen 测试需 LLVM 14 验证 |
| Coherence | ✅ Confirmed | design.md 8 decisions 与 tasks.md 8 batches 一致；跨 batch 17 显式依赖 |

**总体结论**: Pending DP-3 批准 + 实施完成后回填

## Review Gates

**强制审查点**: 每个 Batch 完成后进入下一 Batch 前必须：

1. 运行 `cargo check --workspace`，零警告
2. 运行 `cargo test --workspace`，无新增失败
3. 验证本 Batch 完成标准（见上表）
4. `git diff` 检查无意外文件改动

**阻塞类别**:
- 编译错误（`cargo check` 失败）
- 任何新引入的测试失败
- LLVM IR 缺少关键指令（invoke / landingpad / unreachable）
- 新引入的 `unwrap()`、`as any`、`@ts-ignore` 等反模式
- `cargo clippy` 出现新警告
- 二进制膨胀超过 1.5MB

## Escalation Rules

- **何时回退到 `specifying`**:
  - 任一 Batch 的 Acceptance Criteria 失败 3 次或以上
  - scope 出现未预计的新需求
  - contract 与任务实现实际偏离超过 20%
- **何时回退到 `bridging`**:
  - 单 Batch 内发现新的技术债需要在 contract 中追加 Requirements
  - design.md 决策需要修订（如选型变化）
- **何时不得继续实现**:
  - LLVM 14 环境不可用且 codegen tests 不能运行时（暂停对应任务，其余可继续）
  - 现有 example 回归超过 5%（暂停，回退到 `specifying`）
  - 工作窃取调度器出现 data race（暂停 Batch 2.1，回退到 `specifying`）

## Handoff Rules

- Batch 1（1.1-1.4）→ Batch 2（2.1-2.2）→ Batch 3 → Batch 4 顺序执行
- Batch 1 内 4 个 sub-batch 可并行（GC / 链接 / T9 / trait）
- Batch 2 内 2 个 sub-batch 可并行（await / landing pad）
- 任一 Batch 失败：停下，记录失败证据，回退到 `specifying`
- 阶段 1 完成后做一次阶段评审（DP-3.5 中期 gate），陛下确认后再启动阶段 2

## Ambiguity Flags (Resolved)

- ✅ GC 切换是编译时还是运行时？已确认 **编译时**（`--gc` flag）
- ✅ 运行时库是静态还是动态链接？已确认 **静态**（`libruyi_runtime.a`）
- ✅ `RangeError` 修复是改 stdlib 还是改类型检查器？已确认 **改 stdlib 构造函数**
- ✅ trait 表用 HashMap 还是 AST 遍历？已确认 **HashMap**
- ✅ ruyi_await 用 stackless 还是 stackful？已确认 **stackless**
- ✅ 工作窃取调度器自研还是用 crate？已确认 **`crossbeam-deque`**
- ✅ spawn 返回 JoinHandle 吗？已确认 **fire-and-forget**
- ✅ stdlib 审查单独 change 吗？已确认 **并入本 change T-1.3.3**
- ✅ v0.2-codegen-gaps 处置？已确认 **归档至 `docs/archive/`**（待本 change 启动前完成）
- ✅ fix-try-catch-invoke 状态？已确认 **本 change 启动前先归档**

## Contract Deviations

### DEV-001: `driver.rs` 微小扩展（Batch 1.1 T-1.1.4 必须）

**Deviation Date**: 2026-07-10
**Status**: ✅ Accepted (DP-3.5 陛下批准)
**Affected Task**: T-1.1.4
**Contract Clause Affected**: "MUST NOT modify `driver.rs`" (Scope Fence)

**Reason**: T-1.1.4 要求 CLI `--gc=real` 实际驱动 codegen 切换（验收："编译 `examples/fib.ry --gc=real` 断言 IR 含 `declare @ruyi_gc_alloc`"）。要让 `GcMode` 从 CLI 流向 codegen，必然经 `driver.rs`。

**Scope of Change**（最小化）:

1. `CompileOptions` 新增 `pub gc_mode: GcMode` 字段（含 `Default::default()` 默认 Stub）
2. `driver.rs::compile_program` 一行 `CodeGenerator::with_gc_mode(..., options.gc_mode)` 调用

**Verification**:

- 兼容现有 API（`Driver::new` 签名不变）
- 0 回归（`cargo test --workspace --lib` → 229 passed, 0 failed）
- 0 新警告（baseline 70 = 当前 70）
- 6/6 IR-level 测试 PASS（4 CLI + 2 IR-level，含 `#[ignore]`）

**Mitigation**:

- Batch 1.1 commit message (`ac92134`) 明确标注此 deviation
- 后续 batch 不应再次扩展 `driver.rs`，除非同样必要（如 Batch 1.2 cc_alloc provider 链接）
- contract 后续 review 时正式修订 scope fence

---

## Approval Gate (DP-3)

需用户明确批准后进入 `approved-for-build` 状态。批准后：

```bash
ssf state set changes/v0.5.5-residual-fixes dp_3_result "approved: 8 batches / 23 tasks / 23 mapped requirements / Scope fence 13 items"
ssf state set changes/v0.5.5-residual-fixes dp_3_timestamp $(date -u +%Y-%m-%dT%H:%M:%SZ)
```

---

**请确认 (DP-3)**: 以上契约是否符合预期？批准后立即进入执行阶段（`approved-for-build`）。