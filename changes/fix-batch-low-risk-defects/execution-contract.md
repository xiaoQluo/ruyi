# 执行合同：fix-batch-low-risk-defects

> **Change**: fix-batch-low-risk-defects | **Branch**: `dev/fix-batch-low-risk-defects`
> **Workflow**: full | **Parallel Batches**: 4

## Intent Lock

- **变更名称**：fix-batch-low-risk-defects
- **要解决的问题**：v0.5.9 缺陷审查中优先级 2-5 的 4 类低风险缺陷：`allow_partial_codegen` 全局覆盖静默吞没用户代码 codegen 错误；3 个类型检查器测试断言错误（`pending-issues.md` #1-#3）；12 条 codegen "not yet supported" 错误路径阻断语言特性；路线图文档滞后 5 版本。
- **范围内**：① `driver.rs` allow_partial_codegen 条件化（stdlib-only）；② 修复 3 个测试断言（checker.rs / patterns.rs / types.rs）；③ 实施 codegen/expr.rs 中全部 12 条路径 + decl.rs 1 条路径；④ 更新 `roadmap.md` + `roadmap-zh.md` 至 v0.5.9。
- **范围外**：异常 unwinder（Change A）、Mutex 迁移、CI/CD、tag 补打、proptest、fuzzing、pending-issues #5/#6。

## Approved Behavior

### 已批准需求摘要（10 项）

| 编号 | 特性项 | 行为摘要 |
|------|--------|----------|
| R1 | allow_partial_codegen 范围 | `driver.rs` 传递 `stdlib_item_count` 给 CodeGenerator；用户代码 codegen 错误不再被静默忽略 |
| R2 | allow_partial_codegen 回归 | 所有已有通过测试保持通过；`cargo test -p ruyic --test codegen` 零新增失败 |
| R3 | 匿名函数 codegen | `fn(x) { body }` 编译为 `__anon_{counter}`，复用箭头函数路径 |
| R4 | 异步箭头 codegen | `async (x) => expr` 编译为 `__async_arrow_{counter}`，复用 async fn 路径 |
| R5 | 嵌套成员访问 | `a.b.c` 递归 GEP + load |
| R6 | 间接调用 | 函数指针 → `build_indirect_call` |
| R7 | Spread 参数（4 处） | 统一 `unpack_spread_args`：`__builtin_array_get` 逐元素解包 |
| R8 | 复合赋值 | `x += expr` → load-operate-store（复用 `compile_binary_op`） |
| R9 | 复杂赋值 | `arr[i] = val` → `__builtin_array_set` |
| R10 | 复杂 new 表达式 | 非标识符 callee → 先编译表达式再分配构造 |

### 关键场景

- **R1** 用户代码含 `a.b.c.d`（嵌套成员访问）→ 报错含文件位置，exit ≠ 0
- **R3** `let double = fn(x) { return x * 2; }; double(5)` → `10`
- **R7** `fn sum(a,b,c) { return a+b+c; }; sum(arr[0], arr[1], arr[2])` — spread 相关测试通过
- **R8** `let x = 5; x += 3;` → `x` 为 `8`
- **R10** `throw Error.new("msg")` → 编译通过（此前被 "Complex new expressions" 阻断）

### 验收检查（DP-1 6 项，摘自 proposal.md）

1. `make check` 通过，零新增 clippy 警告
2. `cargo test -p ruyic --test typechecker` 中 3 个原失败测试全部通过
3. 12 条 codegen 路径全部移除 `"not yet supported"` 错误返回（`grep -rn 'not yet supported' crates/ruyic/src/codegen/` 零结果）
4. `allow_partial_codegen` 仅 stdlib 编译时为 `true`
5. 路线图文档 v0.5.5-v0.5.9 已完成项标记 ✅；两文件 `grep '✅'` 一致
6. 零回归——`cargo test -p ruyic --test codegen` 全部通过

## Design Constraints

- **架构约束**：
  - 4 个并行 Batch，内部 sub-batch 3a/3b/3c/3d 可并行
  - Batch 3b 依赖 Task 3a.3（复合赋值上下文——同文件）
  - `allow_partial_codegen` 通过 `CodeGenerator::with_gc_mode_and_stdlib_count()` 新增构造方法传递 stdlib 项计数，非侵入式（不修改 AST 类型）
  - Spread 参数统一通过 `unwrap_spread_args` 公共函数实现（一处定义，4 处调用）
  - 匿名函数和异步箭头复用现有 codegen 路径（`__anon_` / `__async_arrow_` 命名约定）
- **接口约束**：
  1. **Batch 2 → Batch 3**：`stdlib_item_count: usize` 通过 CodeGenerator 构造方法传递
  2. **Batch 3c → Batch 3c**：`fn unpack_spread_args(ctx, args) -> Result<Vec<BasicMetadataValueEnum>, String>` — 公共解包函数
- **依赖约束**：LLVM 14；Rust 2021；clippy zero-warning；不引入新外部 crate；Javadoc 保留

## Task Batches

### Batch 1: Typechecker Test Fixes（4 tasks，Independent）

| Task | File | TDD |
|------|------|-----|
| 1.1 | `checker.rs:172` — fix `test_check_match_statement` | ✅ 5-step |
| 1.2 | `patterns.rs:266` — fix `test_bool_patterns_with_wildcard`（反转断言） | ✅ 5-step |
| 1.3 | `types.rs:652` — fix `test_from_annotation_generic`（更新期望值） | ✅ 5-step |
| 1.4 | `pending-issues.md` — 标记 #1-#4 已修复 | — |

### Batch 2: Driver allow_partial_codegen（2 tasks，Independent）

| Task | File | TDD |
|------|------|-----|
| 2.1 | `generator.rs` — 新增 `with_gc_mode_and_stdlib_count` + `stdlib_item_count` 字段 | ✅ 5-step |
| 2.2 | `driver.rs:570` — 传递 `stdlib_item_count`，移除无条件 `allow_partial_codegen = true` | ✅ 5-step |

### Batch 3: Codegen 12 Paths（13 tasks）

**Sub-Batch 3a: Simple Expressions（4 tasks，Independent）**

| Task | File:Line | Feature |
|------|-----------|---------|
| 3a.1 | `expr.rs:377` | 匿名函数 |
| 3a.2 | `expr.rs:387` | 异步箭头函数 |
| 3a.3 | `expr.rs:2563` | 复合赋值（`+=`/`-=`/`*=`/`/=`/`%=`） |
| 3a.4 | `expr.rs:2336` | 间接调用 |

**Sub-Batch 3b: Access & Assignment（3 tasks，Depends on: 3a.3）**

| Task | File:Line | Feature |
|------|-----------|---------|
| 3b.1 | `expr.rs:2236` | 嵌套成员访问 |
| 3b.2 | `expr.rs:2621` | 复杂赋值（IndexAccess） |
| 3b.3 | `expr.rs:2974` | 复杂 new 表达式 |

**Sub-Batch 3c: Spread Arguments（5 tasks，Depends on: 3c.1）**

| Task | File:Line | Feature |
|------|-----------|---------|
| 3c.1 | `expr.rs`（新函数） | `unpack_spread_args` 公共函数 |
| 3c.2 | `expr.rs:2405` | Spread site 1（函数调用） |
| 3c.3 | `expr.rs:2496` | Spread site 2（函数调用） |
| 3c.4 | `expr.rs:2995` | Spread site 3（构造器） |
| 3c.5 | `expr.rs:3044` | Spread site 4（super 构造器） |

**Sub-Batch 3d: Complex Pattern（1 task，Independent）**

| Task | File:Line | Feature |
|------|-----------|---------|
| 3d.1 | `decl.rs:75` | 复杂模式绑定（Array/Object 解构） |

**Task 3.5**: Final Codegen Regression Gate（`cargo test -p ruyic --test codegen` 全量 + grep 验证）

### Batch 4: Roadmap Docs（2 tasks，Independent）

| Task | File | Work |
|------|------|------|
| 4.1 | `roadmap.md` | 更新至 v0.5.9：版本表、✅ 标记、Current State Assessment |
| 4.2 | `roadmap-zh.md` | 与 EN 对齐更新 |

## Test Obligations

- **TDD 核心边界**：每 task 前写测试 → 确认 RED → 实现 → 确认 GREEN → 回归验证
- **回归敏感区**：`cargo test -p ruyic --test codegen`（codegen 全量）、`cargo test -p ruyic --test typechecker`（typechecker 全量）
- **新增测试**：Batch 3 每个 task 至少 1 个新增 codegen 测试
- **跨 change 依赖**：Batch 3b.3（复杂 new）完成后，Change A 的 `try_catch_invoke` 测试解除 "Complex new expressions" 阻断

## Review Gates

| Gate | 时机 | 验证内容 |
|------|------|----------|
| G1 | Batch 1 完成 | `cargo test -p ruyic --lib`（typechecker 测试通过） |
| G2 | Batch 2 完成 | `cargo test -p ruyic --test codegen`（无回归）+ 手动验证用户代码 codegen 错误不再隐藏 |
| G3 | Batch 3 完成 | `grep -rn 'not yet supported' crates/ruyic/src/codegen/` 零结果 + codegen 全量通过 |
| G4 | Batch 4 完成 | `diff <(grep '✅' roadmap.md) <(grep '✅' roadmap-zh.md)` 一致 |
| G5 | FV 完成后 | `make check` + `cargo test --workspace` 全量通过 |

## Escalation Rules

| 层级 | 触发条件 | 动作 |
|------|----------|------|
| 1: Specifying | 某条 codegen 路径无法在当前架构下实现 | 降级为更精确的错误诊断（含文件位置），标记为 deferred，不阻塞其他路径 |
| 2: Bridging | `make check` 出现新增 clippy 警告 | 暂停，修复警告源，重新运行 |
| 3: Stop | `cargo test -p ruyic --test codegen` 出现回归（previously-passing 测试失败） | 停止全部编辑，git diff 定位变更，回滚或修复 |
