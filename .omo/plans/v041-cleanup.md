# v0.4.1 遗留问题清理工作计划

## TL;DR

> **Quick Summary**: 解决 v0.2/v0.3/v0.4 全部 11 项未完成特性，覆盖代码生成（6项）、运行时（4项）、类型检查（1项），按 P0→P1→P2 优先级分 3 波并行执行。
>
> **Deliverables**:
> - for/for-in/for-of 循环代码生成
> - break/continue 代码生成
> - 可选链 (`?.`) + 计算成员访问 (`obj[expr]`)
> - 模板字面量代码生成
> - BigInt 字面量代码生成
> - match 语句代码生成（全新实现）
> - async/await 真正异步（`ruyi_await` 接入调度器）
> - 异常 landing pad 对接（`__cxa_*` 符号）
> - async GC 根追踪（GenerationalCollector 支持）
> - 线程本地 GC 堆
> - `impl Trait for 内置类型`（string/int/float/bool）
>
> **Estimated Effort**: Large (11 features, most greenfield)
> **Parallel Execution**: YES — 4 waves (+ 1 prerequisite)
> **Critical Path**: T0 → T1 → T2 → T6 → T7 → T8 → T10

---

## Context

### Original Request
用户要求按优先级在 v0.4.1 版本解决 v0.5 以前（v0.2/v0.3/v0.4）所有遗留问题。

### Interview Summary
**Key Discussions**:
- 范围确认：仅 v0.2/v0.3/v0.4（11 项），排除 v0.5 标准库扩展
- 测试策略：实现后补测，不做 TDD
- 所有任务必须有 Agent-Executed QA 场景

**Research Findings**:
- LLVM 14 未安装：代码生成/集成测试无法运行，需使用 `--emit-llvm` + IR 文本断言验证
- 单元测试（lexer/parser/typechecker）无需 LLVM 可运行
- `patterns.rs` 全为空操作存根 — match 语句是全新实现
- `ruyi_await` 为空操作透传，调度器已存在但未接线
- `register_async_roots` 仅支持 `MarkSweepCollector`，实际使用 `GenerationalCollector`
- 异常运行时 `ruyi_throw` 使用 `panic!` 而非 `_Unwind_RaiseException`

### Metis Review
**Identified Gaps** (addressed):
- 大多数特性是绿地实现而非存根接线 — 已按实现难度重新分级
- 无限 LLVM 验证策略缺失 — 已定义 `--emit-llvm` + IR 文本断言方案
- 异常 landing pad 可能需要代码生成变更 — 增加 T6 审计任务
- Match 语句风险最高 — 提升为独立任务，必要时可降级
- `loop_stack` 已存在但未使用 — T1 先实现循环，T2 接入 break/continue

---

## Work Objectives

### Core Objective
解决 Ruyi 编译器 v0.2-v0.4 版本规划中全部 11 项未完成/部分完成的特性，使语言核心功能完整可用。

### Concrete Deliverables
| 领域 | 文件 | 特性 |
|------|------|------|
| 代码生成 | `codegen/stmt.rs` | for/for-in/for-of 循环, break/continue, match 语句 |
| 代码生成 | `codegen/expr.rs` | 可选链(?.), 计算成员, 模板字面量, BigInt 字面量 |
| 运行时 | `ruyi_runtime/src/async_runtime.rs` | async await 真正异步接线 |
| 运行时 | `ruyi_runtime/src/exception/` | `__cxa_begin_catch`/`__cxa_end_catch` 对接 |
| 运行时 | `ruyi_runtime/src/gc_exports.rs`, `gc/` | GC 根追踪 + 线程本地堆 |
| 类型检查 | `typechecker/traits.rs` | `impl Trait for 内置类型` |

### Definition of Done
- [ ] 所有 11 项特性均可在 `.ry` 文件中使用并通过 `ruyic --check` 类型检查
- [ ] 代码生成任务通过 `--emit-llvm` 输出 LLVM IR 验证
- [ ] 运行时任务通过 `cargo test -p ruyi_runtime` 验证
- [ ] 现有 `cargo check --workspace` 零回归错误
- [ ] Agent QA 场景全部通过并产生证据文件

### Must Have
- 完整的 for/for-in/for-of 循环代码生成（P0）
- 真正的 async/await 异步执行（P0）
- 异常传播的 landing pad 对接（P0）
- 可选链 `?.` 和计算成员 `obj[expr]`（P0）
- break/continue 代码生成（P1）
- match 语句代码生成（P1）
- 模板字面量代码生成（P1）

### Must NOT Have (Guardrails)
- **MUST NOT**: 新增 v0.5+ 语言特性（标记模板、`??` 运算符、带标签 break/continue）
- **MUST NOT**: 重构已有工作代码（`decl.rs`、`expr.rs` 中已实现的表达式）
- **MUST NOT**: 引入新依赖（锁定 `inkwell 0.2`、LLVM 14）
- **MUST NOT**: 修改 AST/HIR 结构除非该特性当前解析即为缺失
- **MUST NOT**: 优化循环代码生成（不展开、不向量化）
- **MUST NOT**: 在 `match` 实现中做穷尽性检查（typechecker 职责，非 codegen）
- **MUST NOT**: 为 BigInt 添加运算符支持（仅字面量）
- **MUST NOT**: 添加异步组合子（`Promise.all` 等，属 v0.5+）
- **MUST NOT**: 跨 crate 自由修改（codegen 不改 runtime，runtime 不改 codegen，除非任务明确要求）
- **MUST NOT**: "顺便"修复相邻代码——无关变更需独立任务

---

## Verification Strategy

### Test Decision
- **Infrastructure exists**: YES (`cargo test` + integration tests)
- **Automated tests**: Tests-after (not TDD)
- **Framework**: Rust built-in `#[test]` + `cargo test`
- **LLVM limitation**: 代码生成/集成测试需 LLVM，无法运行时使用 `--emit-llvm` IR 文本断言

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### No-LLVM Verification Protocol (MANDATORY for codegen tasks)
由于当前环境无 LLVM 14，代码生成验证采用以下策略：
1. `cargo check -p ruyic` — 确认编译无错误
2. `cargo check -p ruyi_runtime --no-default-features` — 确认运行时编译
3. `ruyic example.ry --emit-llvm -o /dev/stdout` — 生成 LLVM IR 文本
4. 使用 `grep` 在 IR 输出中匹配关键模式（如 `br i1`、`invoke`、`alloca`）
5. `ruyic example.ry --emit-ast` — 确认 AST 解析正确
6. `ruyic example.ry --check` — 确认类型检查通过

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Codegen tasks**: `--emit-llvm` + IR pattern matching (Bash)
- **Runtime tasks**: `cargo test` specific test functions (Bash)
- **Typechecker tasks**: `ruyic --check` + expected diagnostics (Bash)

---

## Execution Strategy

### Parallel Execution Waves

```
Prerequisite (START HERE — branch setup):
└── T0: 创建 dev/v0.4.1 分支 + 版本号更新 [quick]

Wave 1 (After T0 — isolated codegen, max parallelism):
├── T1: for/for-in/for-of 循环代码生成 [deep]
├── T2: break/continue 代码生成 (depends: T1) [quick]
├── T3: 可选链 (?.) + 计算成员代码生成 [deep]
├── T4: 模板字面量代码生成 [quick]
└── T5: BigInt 字面量代码生成 [quick]

Wave 2 (After Wave 1 — runtime + typechecker, MAX PARALLEL):
├── T6: 审计 try/catch 代码生成 (前置: 异常 landing pad) [quick]
├── T7: 异常 landing pad 对接 (depends: T6) [deep]
├── T8: async/await 真正异步接线 [deep]
├── T9: async GC 根 GenerationalCollector 支持 [unspecified-high]
└── T10: impl Trait for 内置类型 [quick]

Wave 3 (After Wave 2 — 高风险 + 增强项):
├── T11: match 语句代码生成 (全新实现, 最高风险) [deep]
└── T12: 线程本地 GC 堆 [unspecified-high]

Wave FINAL (After ALL tasks — 4 parallel reviews):
├── F1: Plan Compliance Audit [oracle]
├── F2: Code Quality Review [unspecified-high]
├── F3: Real Manual QA [unspecified-high]
└── F4: Scope Fidelity Check [deep]
```

Critical Path: T0 → T1 → T2 → T6 → T7 → T8 → T10

---

## TODOs

- [x] 0. **创建 dev/v0.4.1 分支 + 版本号更新** (前置)

  **What to do**:
  - 从当前 `dev/v0.4` 分支创建 `dev/v0.4.1` 分支：
    ```bash
    git checkout -b dev/v0.4.1
    ```
  - 更新版本号：
    - `Cargo.toml`: workspace `version = "0.4.1"`
    - `crates/ruyic/src/main.rs`: `#[command(version = "0.4.1")]`
  - 提交版本号变更：
    ```bash
    git add Cargo.toml crates/ruyic/src/main.rs
    git commit -m "chore: bump version to 0.4.1"
    ```
  - 确认分支已创建：`git branch --show-current` → `dev/v0.4.1`

  **Must NOT do**:
  - 不推送到远程（本地开发，合并时再 push）
  - 不修改除版本号外的任何代码

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 纯 git 操作 + 两处版本号替换，无编译依赖

  **Parallelization**:
  - **Can Run In Parallel**: NO（阻塞所有后续任务）
  - **Parallel Group**: Prerequisite
  - **Blocks**: T1-T12（所有后续任务）
  - **Blocked By**: None

  **References**:
  - `Cargo.toml:3` — workspace `version = "0.4.0"`（当前值）
  - `crates/ruyic/src/main.rs:18` — `#[command(version = "0.4.0")]`（当前值）
  - `AGENTS.md` — 版本切换检查清单 + 分支策略

  **Acceptance Criteria**:
  - [ ] `git branch --show-current` 输出 `dev/v0.4.1`
  - [ ] `grep 'version = "0.4.1"' Cargo.toml` 匹配成功
  - [ ] `grep 'version = "0.4.1"' crates/ruyic/src/main.rs` 匹配成功

  **QA Scenarios**:

  ```
  Scenario: branch created and version updated
    Tool: Bash
    Steps:
      1. git branch --show-current
      2. grep '0.4.1' Cargo.toml | head -1
      3. grep '0.4.1' crates/ruyic/src/main.rs | head -1
    Expected Result: 当前分支 = dev/v0.4.1，两处版本号均为 0.4.1
    Evidence: .sisyphus/evidence/task-0-branch.txt
  ```

  **Commit**: YES
  - Message: `chore: bump version to 0.4.1`
  - Files: `Cargo.toml`, `crates/ruyic/src/main.rs`

- [x] 1. **for/for-in/for-of 循环代码生成** (P0)

  **What to do**:
  - 在 `codegen/stmt.rs` 的 `compile_stmt` 中添加 `Statement::For` / `Statement::ForIn` / `Statement::ForOf` 分支
  - `for (let i = 0; i < n; i = i + 1) { body }`：生成 init → cond_bb → body → update → cond_bb 循环结构
  - `for (let key in obj) { body }`：调用运行时 `ruyi_obj_keys` 获取键数组，遍历键
  - `for (let item of iterable) { body }`：调用迭代器协议 `.iter()` → `.next()` 循环
  - 将循环的 `(end_bb, cond_bb)` 推入 `ctx.loop_stack`（为 T2 break/continue 做准备）
  - 参考已有 `compile_while` 实现（`stmt.rs:115-156`）的 BasicBlock 布局模式

  **Must NOT do**:
  - 不实现带标签的 break/continue
  - 不优化循环展开或向量化
  - 不修改 `parser/ast.rs` 中的 AST 结构（for/for-in/for-of 已解析）

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 需要理解 LLVM IR BasicBlock 控制流 + Ruyi 迭代器协议，逻辑复杂但范围明确
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (与 T3, T4, T5 并行)
  - **Parallel Group**: Wave 1
  - **Blocks**: T2
  - **Blocked By**: None

  **References**:
  - `crates/ruyic/src/codegen/stmt.rs:115-156` — `compile_while` 循环布局模式（init/cond/body/end blocks）
  - `crates/ruyic/src/codegen/generator.rs:53-56` — `loop_stack: Vec<(BasicBlock, BasicBlock)>` 定义
  - `crates/ruyic/src/parser/ast.rs` — `Statement::For`/`ForIn`/`ForOf` 的 AST 节点结构
  - `crates/ruyic/src/codegen/async_codegen.rs:35-58` — 已有 For/ForIn/ForOf 的 await 计数逻辑（参考 AST 遍历方式）

  **Acceptance Criteria**:
  - [ ] `cargo check -p ruyic` 无编译错误
  - [ ] `ruyic examples/array.ry --check` 通过（含 for-of 遍历）

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: C-style for loop generates correct IR
    Tool: Bash
    Preconditions: examples/array.ry 包含 `for (let i = 0; i < 3; i = i + 1) { print(i); }`
    Steps:
      1. ruyic examples/array.ry --emit-llvm -o /tmp/test_for.ll
      2. grep 'br' /tmp/test_for.ll | head -20
      3. grep 'phi' /tmp/test_for.ll
    Expected Result: IR 包含至少 3 个 basic block（init/cond/body/end 循环结构），存在 `br` 分支指令
    Failure Indicators: 无 `br` 指令或仅有单个 basic block
    Evidence: .sisyphus/evidence/task-1-for-loop.ll.txt

  Scenario: for-in loop compiles without error
    Tool: Bash
    Preconditions: 创建 `test_for_in.ry` 含 `let obj = {a: 1, b: 2}; for (let k in obj) { print(k); }`
    Steps:
      1. ruyic test_for_in.ry --check
      2. echo $?
    Expected Result: 退出码 0（类型检查通过）
    Evidence: .sisyphus/evidence/task-1-for-in.txt

  Scenario: for loop with break compiles (cross-check with T2)
    Tool: Bash
    Preconditions: 创建 `test_for_break.ry` 含 `for (let i = 0; i < 10; i = i + 1) { if (i > 5) { break; } }`
    Steps:
      1. ruyic test_for_break.ry --check
      2. ruyic test_for_break.ry --emit-llvm | grep 'br'
    Expected Result: IR 包含无条件分支跳转到循环结束块
    Evidence: .sisyphus/evidence/task-1-for-break.ll.txt
  ```

  **Commit**: YES (groups with G1)
  - Message: `feat(codegen): implement for/for-in/for-of loop codegen`
  - Files: `crates/ruyic/src/codegen/stmt.rs`

- [x] 2. **break/continue 代码生成** (P1)

  **What to do**:
  - 在 `codegen/stmt.rs` 的 `compile_stmt` 中添加 `Statement::Break` / `Statement::Continue` 分支
  - `break`：从 `ctx.loop_stack` 取栈顶 `(end_bb, _)`，生成无条件 `br end_bb`
  - `continue`：从 `ctx.loop_stack` 取栈顶 `(_, cond_bb)`，生成无条件 `br cond_bb`
  - 处理嵌套循环：break/continue 作用于最近的循环（栈顶），无需标签支持
  - 错误处理：如果 `loop_stack` 为空（在循环外使用 break/continue），生成编译错误 `DiagnosticKind::BreakOutsideLoop`

  **Must NOT do**:
  - 不实现带标签的 break/continue（如 `break outer`）
  - 不修改 `loop_stack` 的定义结构

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 单文件修改，逻辑简单（从已有栈取 BB 跳转），无新概念
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO（依赖 T1 完成——需要 `loop_stack` 被 push）
  - **Parallel Group**: Wave 1 (串行尾)
  - **Blocks**: None
  - **Blocked By**: T1

  **References**:
  - `crates/ruyic/src/codegen/generator.rs:53-56` — `loop_stack` 定义
  - `crates/ruyic/src/codegen/stmt.rs:126-153` — `compile_while` 中 push/pop loop_stack 的模式
  - `crates/ruyic/src/diagnostics/codes.rs` — `DiagnosticKind::BreakOutsideLoop` 或类似错误码（若不存在需新增）

  **Acceptance Criteria**:
  - [ ] `cargo check -p ruyic` 无编译错误
  - [ ] 循环内 break 生成 `br end_bb`（IR 验证）
  - [ ] 循环外 break 产生编译错误

  **QA Scenarios**:

  ```
  Scenario: break in for loop jumps to end block
    Tool: Bash
    Preconditions: test_for_break.ry 含 `for (let i = 0; i < 10; i = i + 1) { if (i > 5) { break; } print(i); }`
    Steps:
      1. ruyic test_for_break.ry --emit-llvm -o /tmp/test_break.ll
      2. grep -A2 'br label' /tmp/test_break.ll | head -20
    Expected Result: 至少有一条 `br label %end_block_N`（无条件跳转到循环结束块）
    Evidence: .sisyphus/evidence/task-2-break.ll.txt

  Scenario: break outside loop produces error
    Tool: Bash
    Preconditions: 创建 test_break_err.ry 含 `fn main() { break; }`
    Steps:
      1. ruyic test_break_err.ry --check 2>&1
    Expected Result: stderr 包含 "break outside loop" 或类似错误
    Failure Indicators: 退出码 0（意外通过）
    Evidence: .sisyphus/evidence/task-2-break-err.txt
  ```

  **Commit**: YES (groups with G1)
  - Message: `feat(codegen): implement break/continue codegen`
  - Files: `crates/ruyic/src/codegen/stmt.rs`

- [x] 3. **可选链 (`?.`) + 计算成员访问 (`obj[expr]`) 代码生成** (P0)

  **What to do**:
  - 在 `codegen/expr.rs` 的 `compile_member_access` 中扩展支持：
    - **可选链 `obj?.prop`**：检查 `Expr::Member { optional: true, ... }`，在 GEP + load 前插入 null 检查。若对象为 null，短路返回 null 而不访问属性
    - **计算成员 `obj[expr]`**：处理 `MemberProperty::Expr`，编译键表达式，生成运行时 `ruyi_obj_get` 调用
  - 短路逻辑：`build_is_null(obj_ptr)` → `build_conditional_branch` → null 路径返回 null / 非 null 路径执行 GEP
  - 深层链 `a?.b?.c`：多层嵌套 Member 表达式，每层独立短路

  **Must NOT do**:
  - 不实现 nullish coalescing (`??`)
  - 不实现可选调用 `obj?.()` 或可选索引 `arr?.[0]`
  - 不修改成员访问的已有实现（简单 `obj.prop` 保持不变）

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 需要 LLVM IR phi 节点处理（短路分支汇合），控制流较复杂
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (与 T1, T4, T5 并行)
  - **Parallel Group**: Wave 1
  - **Blocks**: None
  - **Blocked By**: None

  **References**:
  - `crates/ruyic/src/codegen/expr.rs:174-261` — `compile_member_access` 当前实现（简单 `obj.prop`）
  - `crates/ruyic/src/codegen/expr.rs:93` — `Expr::Member` 在 `compile_expr` 中的路由
  - `crates/ruyic/src/parser/ast.rs` — `Expr::Member { optional: bool, ... }` 的 `optional` 字段定义
  - `crates/ruyic/src/codegen/builtins.rs` — 参考 `build_gc_alloc` 等辅助函数的使用模式

  **Acceptance Criteria**:
  - [ ] `cargo check -p ruyic` 无编译错误
  - [ ] `obj?.prop` 对 null 对象生成短路跳转
  - [ ] `obj[expr]` 生成运行时调用

  **QA Scenarios**:

  ```
  Scenario: optional chaining on non-null object works
    Tool: Bash
    Preconditions: 创建 test_opt_chain.ry 含 class A { x: int; } 及 obj?.x 访问
    Steps:
      1. ruyic test_opt_chain.ry --check
    Expected Result: 退出码 0（类型检查通过）
    Evidence: .sisyphus/evidence/task-3-opt-chain-check.txt

  Scenario: optional chaining generates null check in IR
    Tool: Bash
    Preconditions: test_opt_chain.ry
    Steps:
      1. ruyic test_opt_chain.ry --emit-llvm -o /tmp/test_oc.ll
      2. grep -E 'icmp.*null|isnull' /tmp/test_oc.ll
    Expected Result: IR 包含 null 比较指令
    Evidence: .sisyphus/evidence/task-3-opt-chain-ll.txt

  Scenario: computed member access works
    Tool: Bash
    Preconditions: 创建 test_computed.ry 含 `let key = "x"; let val = obj[key];`
    Steps:
      1. ruyic test_computed.ry --check
    Expected Result: 退出码 0
    Evidence: .sisyphus/evidence/task-3-computed-check.txt
  ```

  **Commit**: YES (groups with G1)
  - Message: `feat(codegen): implement optional chaining and computed member access`
  - Files: `crates/ruyic/src/codegen/expr.rs`

- [x] 4. **模板字面量代码生成** (P1)

  **What to do**:
  - 在 `codegen/expr.rs` 的 `compile_expr` 中添加 `Expr::TemplateLiteral` 分支
  - 将模板字面量 `` `Hello ${name}, you are ${age}` `` 降低为字符串拼接链：`"Hello " + name + ", you are " + age`
  - 利用已有的 `compile_add` (expr.rs:297-355) 中 `ruyi_str_concat` 字符串拼接支持
  - 处理纯字符串段（无嵌入表达式）时跳过拼接直接返回字符串常量
  - 空模板 `` ```` → 生成空字符串

  **Must NOT do**:
  - 不实现标记模板（tagged templates: `` tag`hello` ``）
  - 不在 codegen 层处理转义（转义已在 lexer 中处理）

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 降低到已有功能（字符串拼接），逻辑简单，单函数实现
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (与 T1, T3, T5 并行)
  - **Parallel Group**: Wave 1
  - **Blocks**: None
  - **Blocked By**: None

  **References**:
  - `crates/ruyic/src/codegen/expr.rs:297-355` — `compile_add` 字符串拼接实现（`ruyi_str_concat`）
  - `crates/ruyic/src/parser/ast.rs` — `Expr::TemplateLiteral { parts: Vec<TemplatePart> }` AST 结构
  - `crates/ruyic/src/typechecker/inference.rs:650` — 模板字面量的类型推断（参考 parts 结构）

  **Acceptance Criteria**:
  - [ ] `cargo check -p ruyic` 无编译错误
  - [ ] 模板字面量生成对 `ruyi_str_concat` 的调用

  **QA Scenarios**:

  ```
  Scenario: template literal with interpolation
    Tool: Bash
    Preconditions: 创建 test_template.ry 含 `let name = "World"; let msg = `Hello ${name}!`;`
    Steps:
      1. ruyic test_template.ry --emit-llvm -o /tmp/test_tmpl.ll
      2. grep 'ruyi_str_concat' /tmp/test_tmpl.ll
    Expected Result: IR 包含 `ruyi_str_concat` 调用
    Evidence: .sisyphus/evidence/task-4-template.ll.txt

  Scenario: template literal with no interpolation (pure string)
    Tool: Bash
    Preconditions: 创建 test_template2.ry 含 `let msg = `Hello World`;`
    Steps:
      1. ruyic test_template2.ry --check
    Expected Result: 退出码 0
    Evidence: .sisyphus/evidence/task-4-template2-check.txt
  ```

  **Commit**: YES (groups with G1)
  - Message: `feat(codegen): implement template literal codegen`
  - Files: `crates/ruyic/src/codegen/expr.rs`

- [x] 5. **BigInt 字面量代码生成** (P2)

  **What to do**:
  - 在 `codegen/expr.rs` 的 `compile_expr` 中添加 `Expr::BigIntLiteral` 分支
  - 检查 `Type::BigInt` 在 `codegen/types.rs` 中的 LLVM 映射（当前为 `i8*` 指针）
  - 实现 `compile_bigint_literal(value: String)`：调用运行时 `ruyi_bigint_from_str(value)` 创建 BigInt
  - 在 `codegen/builtins.rs` 声明 `ruyi_bigint_from_str` 外部函数
  - 若运行时尚无 `ruyi_bigint_from_str`，在 `ruyi_runtime/src/` 中添加 C 导出函数（简单存根：分配内存 + 存储字符串）

  **Must NOT do**:
  - 不实现 BigInt 算术运算符（`+`/`-`/`*` 等）
  - 不实现 BigInt 与 int 的隐式转换
  - 不优化 BigInt 存储格式

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 单文件修改 + 简单运行时存根，P2 优先级
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (与 T1, T3, T4 并行)
  - **Parallel Group**: Wave 1
  - **Blocks**: None
  - **Blocked By**: None

  **References**:
  - `crates/ruyic/src/codegen/types.rs` — `Type::BigInt` 的 LLVM 类型映射
  - `crates/ruyic/src/codegen/builtins.rs` — 运行时函数声明模式（参考 `declare_ruyi_spawn` 等）
  - `crates/ruyic/src/parser/ast.rs` — `Expr::BigIntLiteral(String)` AST 节点

  **Acceptance Criteria**:
  - [ ] `cargo check -p ruyic` 无编译错误
  - [ ] `100n` 字面量编译通过 `--check`

  **QA Scenarios**:

  ```
  Scenario: BigInt literal compiles
    Tool: Bash
    Preconditions: 创建 test_bigint.ry 含 `let x = 100n;`
    Steps:
      1. ruyic test_bigint.ry --check
    Expected Result: 退出码 0
    Evidence: .sisyphus/evidence/task-5-bigint-check.txt

  Scenario: BigInt literal generates runtime call
    Tool: Bash
    Preconditions: test_bigint.ry
    Steps:
      1. ruyic test_bigint.ry --emit-llvm -o /tmp/test_bi.ll
      2. grep 'ruyi_bigint' /tmp/test_bi.ll
    Expected Result: IR 包含 `ruyi_bigint_from_str` 调用
    Evidence: .sisyphus/evidence/task-5-bigint-ll.txt
  ```

  **Commit**: YES (groups with G1)
  - Message: `feat(codegen): implement BigInt literal codegen`
  - Files: `crates/ruyic/src/codegen/expr.rs`, `crates/ruyic/src/codegen/builtins.rs`, `crates/ruyi_runtime/src/bigint.rs` (new)

- [x] 6. **审计 try/catch 代码生成** (前置任务，P0)

  **What to do**:
  - 阅读 `codegen/stmt.rs:245-378` 的 `compile_try` 实现
  - 确认代码生成中使用的是 `invoke`（而非 `call`）来调用 try 块内可能抛出的函数
  - 若使用 `call`，记录需要修改的位置——这将成为 T7 异常 landing pad 的依赖
  - 检查 `compile_throw` (stmt.rs:185-242) 中 `ruyi_throw` 的调用方式
  - 检查 `ruyi_runtime/src/exception/runtime.rs` 中 `LandingPadGenerator` 的接口
  - 输出审计报告：`TRY_CATCH_AUDIT.md` 记录发现和建议

  **Must NOT do**:
  - 不做代码修改（仅审计和记录）
  - 不重构 try/catch 实现

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 纯读代码 + 分析报告，无代码修改，快速完成
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (与 Wave 1 所有任务并行——纯读操作)
  - **Parallel Group**: Wave 1/Wave 2 过渡
  - **Blocks**: T7
  - **Blocked By**: None

  **References**:
  - `crates/ruyic/src/codegen/stmt.rs:245-378` — `compile_try` 完整实现
  - `crates/ruyic/src/codegen/stmt.rs:185-242` — `compile_throw` 实现
  - `crates/ruyi_runtime/src/exception/runtime.rs` — ExceptionRuntime + LandingPadGenerator
  - `crates/ruyi_runtime/src/exception/landing_pad.rs` — landing pad 生成器

  **Acceptance Criteria**:
  - [ ] 审计报告 `TRY_CATCH_AUDIT.md` 已创建，回答：
    - try 块内函数调用使用 `invoke` 还是 `call`？
    - `ruyi_throw` 如何与控制流交互？
    - LandingPadGenerator 的接口是否与 codegen 兼容？
    - T7 需要修改哪些文件？

  **QA Scenarios**:
  - N/A（纯审计任务，输出为 Markdown 报告）
  - Evidence: `.sisyphus/evidence/task-6-audit.md`

  **Commit**: NO（审计任务，不产生代码变更）

- [x] 7. **异常 landing pad 对接** (P0)

  **What to do**:
  - 基于 T6 审计结果，将运行时异常处理从 `panic!` 改为真实栈展开
  - 运行时侧（`ruyi_runtime/src/exception/runtime.rs`）：
    - `ruyi_throw`：调用 `_Unwind_RaiseException` (Itanium C++ ABI) 而非 `panic!`
    - `ruyi_begin_catch`：调用 `__cxa_begin_catch` 获取异常对象
    - `ruyi_end_catch`：调用 `__cxa_end_catch` 清理
  - 编译器侧（若 T6 发现需要）：
    - 将 try 块内的 `call` 指令改为 `invoke`，指定 catch/finally landing pad 为异常目标
    - 使用 `inkwell` 的 `build_invoke` API
  - 确保 `finally` 块在异常和正常路径均执行

  **Must NOT do**:
  - 不添加异常堆栈追踪（`backtrace`）
  - 不处理 C++ 异常与 Ruyi 异常的互操作
  - 不修改 `__cxa_*` 的全局状态管理

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 跨 crate（编译器 + 运行时）+ LLVM invoke/landingpad 指令 + Itanium ABI，复杂度高
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (与 T8, T9, T10 并行——各改不同文件)
  - **Parallel Group**: Wave 2
  - **Blocks**: None
  - **Blocked By**: T6

  **References**:
  - `crates/ruyi_runtime/src/exception/runtime.rs:20-65` — 当前 `ruyi_throw`/`ruyi_begin_catch`/`ruyi_end_catch` stubs
  - `crates/ruyi_runtime/src/exception/landing_pad.rs:19-210` — `LandingPadGenerator` 完整实现
  - `crates/ruyic/src/codegen/stmt.rs:245-378` — `compile_try` 代码生成
  - Itanium C++ ABI: `_Unwind_RaiseException`, `__cxa_begin_catch`, `__cxa_end_catch`

  **Acceptance Criteria**:
  - [ ] `cargo check -p ruyi_runtime --no-default-features` 无编译错误
  - [ ] `ruyi_throw` 调用 `_Unwind_RaiseException`（代码审查确认）
  - [ ] `ruyi_begin_catch` / `ruyi_end_catch` 调用 `__cxa_*` 对应函数

  **QA Scenarios**:

  ```
  Scenario: try-catch generates invoke not call (if codegen change needed)
    Tool: Bash
    Preconditions: examples/try_catch.ry
    Steps:
      1. ruyic examples/try_catch.ry --emit-llvm -o /tmp/test_eh.ll
      2. grep 'invoke' /tmp/test_eh.ll
    Expected Result: IR 包含 `invoke` 指令（而非仅 `call`）
    Failure Indicators: 只有 `call` 无 `invoke`（若 T6 确认需要 invoke）
    Evidence: .sisyphus/evidence/task-7-eh-invoke.ll.txt

  Scenario: runtime exception functions exist
    Tool: Bash
    Preconditions: None
    Steps:
      1. grep -n '_Unwind_RaiseException\|__cxa_begin_catch\|__cxa_end_catch' crates/ruyi_runtime/src/exception/runtime.rs
    Expected Result: 至少匹配到 `_Unwind_RaiseException`
    Evidence: .sisyphus/evidence/task-7-eh-runtime.txt
  ```

  **Commit**: YES (groups with G2)
  - Message: `feat(runtime): wire exception landing pads with Itanium C++ ABI`
  - Files: `crates/ruyi_runtime/src/exception/runtime.rs`, `crates/ruyic/src/codegen/stmt.rs` (若需要)

- [x] 8. **async/await 真正异步接线** (P0)

  **What to do**:
  - 将 `ruyi_await` 从 no-op 透传改为真正的调度器挂起
  - 运行时侧（`ruyi_runtime/src/async_runtime.rs:347-360`）：
    - 修改 `ruyi_await`：不再直接返回 future 指针，而是将当前任务挂起
    - 调用 `scheduler.suspend_current(task_id)`，将 future 提交给调度器轮询
    - 当 future 完成时，调度器唤醒任务继续执行
  - 编译器侧（`codegen/async_codegen.rs`）：
    - 验证异步状态机生成正确（已在 v0.3 中实现 `compile_async_fn`）
    - 确保 await 点生成 `ruyi_await` 调用（当前已生成，需确认）
  - 验证 `spawn(fn)` 已在 v0.3 中完成（`async_exports.rs:68-72`），无需额外工作

  **Must NOT do**:
  - 不添加并行异步组合子（`Promise.all`、`race` 等）
  - 不修改调度器的核心算法（工作窃取已实现）
  - 不添加 async 闭包或 async 迭代器

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 异步运行时核心逻辑变更，涉及调度器挂起/唤醒 + future 状态机，需理解完整异步栈
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (与 T7, T9, T10 并行——不同文件)
  - **Parallel Group**: Wave 2
  - **Blocks**: None
  - **Blocked By**: None

  **References**:
  - `crates/ruyi_runtime/src/async_runtime.rs:347-360` — 当前 `ruyi_await` no-op 实现
  - `crates/ruyi_runtime/src/async_runtime.rs:265-269` — `Scheduler::spawn` 实现
  - `crates/ruyi_runtime/src/async_runtime.rs:304-343` — 调度器工作循环
  - `crates/ruyi_runtime/src/async_exports.rs:68-72` — `ruyi_spawn` C 导出（已实现）
  - `crates/ruyic/src/codegen/async_codegen.rs:507-556` — 异步状态机 `ruyi_async_poll` 生成
  - `crates/ruyic/src/codegen/builtins.rs:355-359` — `ruyi_spawn` 声明

  **Acceptance Criteria**:
  - [ ] `cargo check -p ruyi_runtime --no-default-features` 无编译错误
  - [ ] `ruyi_await` 不再直接返回 future 指针（代码审查确认调度器交互）
  - [ ] `ruyi_spawn` + `ruyi_await` 组合可通过测试

  **QA Scenarios**:

  ```
  Scenario: async function compiles with await
    Tool: Bash
    Preconditions: examples/async.ry
    Steps:
      1. ruyic examples/async.ry --check
    Expected Result: 退出码 0
    Evidence: .sisyphus/evidence/task-8-async-check.txt

  Scenario: async IR contains scheduler interaction
    Tool: Bash
    Preconditions: examples/async.ry
    Steps:
      1. ruyic examples/async.ry --emit-llvm -o /tmp/test_async.ll
      2. grep -E 'ruyi_await|ruyi_spawn|ruyi_async_poll' /tmp/test_async.ll
    Expected Result: IR 包含 `ruyi_await` 和 `ruyi_spawn` 调用
    Evidence: .sisyphus/evidence/task-8-async-ll.txt
  ```

  **Commit**: YES (groups with G2)
  - Message: `feat(runtime): wire async await to work-stealing scheduler`
  - Files: `crates/ruyi_runtime/src/async_runtime.rs`

- [x] 9. **async GC 根 GenerationalCollector 支持** (P1)

  **What to do**:
  - 当前 `register_async_roots` 仅接受 `&mut MarkSweepCollector`（`async_runtime.rs:386-415`）
  - 实际活跃 GC 是 `GenerationalCollector`（`gc_exports.rs:22-23`）
  - 为 `GenerationalCollector` 添加 `register_async_roots` 方法
  - 方法签名：`fn register_async_roots(&mut self, tasks: &[(TaskId, Task)])`
  - 扫描所有挂起的异步任务，将任务持有的 GC 指针注册为根
  - 在 `GLOBAL_COLLECTOR` 的 `collect_full()` 调用前调用 `register_async_roots`
  - 或者：创建通用 trait `RootProvider`，让两种收集器都实现

  **Must NOT do**:
  - 不移除 `MarkSweepCollector` 的已有实现（保持向后兼容）
  - 不修改 generational collection 的算法

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: 跨模块修改（async_runtime + gc + gc_exports），需理解 GC 内部结构
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (与 T7, T8, T10 并行——不同模块)
  - **Parallel Group**: Wave 2
  - **Blocks**: None
  - **Blocked By**: None

  **References**:
  - `crates/ruyi_runtime/src/async_runtime.rs:386-415` — 当前 `register_async_roots` for MarkSweepCollector
  - `crates/ruyi_runtime/src/gc_exports.rs:22-23` — `GLOBAL_COLLECTOR: Lazy<Mutex<SendCollector>>`
  - `crates/ruyi_runtime/src/gc/generational.rs:21-29` — `GenerationalCollector` 结构体字段
  - `crates/ruyi_runtime/src/gc.rs:200-224` — `Collector` trait / wrapper

  **Acceptance Criteria**:
  - [ ] `cargo check -p ruyi_runtime --no-default-features` 无编译错误
  - [ ] `GenerationalCollector` 有 `register_async_roots` 方法或等效 trait 实现
  - [ ] 单元测试验证 roots 注册（`cargo test -p ruyi_runtime`）

  **QA Scenarios**:

  ```
  Scenario: generational collector exposes async root registration
    Tool: Bash
    Preconditions: None
    Steps:
      1. grep -n 'register_async_roots' crates/ruyi_runtime/src/gc/generational.rs
    Expected Result: 匹配到方法定义或 trait impl
    Evidence: .sisyphus/evidence/task-9-gc-roots.txt

  Scenario: runtime compiles with generational collector + async roots
    Tool: Bash
    Preconditions: None
    Steps:
      1. cargo check -p ruyi_runtime --no-default-features 2>&1
    Expected Result: 退出码 0
    Evidence: .sisyphus/evidence/task-9-check.txt
  ```

  **Commit**: YES (groups with G2)
  - Message: `feat(runtime): add async GC root registration for GenerationalCollector`
  - Files: `crates/ruyi_runtime/src/gc/generational.rs`, `crates/ruyi_runtime/src/async_runtime.rs`

- [x] 10. **`impl Trait for 内置类型`** (P1)

  **What to do**:
  - 修复 `typechecker/traits.rs` 中 `register_impl()` 的 `type_annotation_name()` 函数
  - 当前仅处理 `TypeAnnotation::Identifier` 和 `TypeAnnotation::Generic`
  - 新增 `TypeAnnotation::Builtin` 处理：映射 `"string"` → `Type::String`、`"int"` → `Type::Int`、`"float"` → `Type::Float`、`"bool"` → `Type::Bool`
  - 更新 `implements()` 函数（`traits.rs:173-181`）：除 `Type::Named`/`Type::Generic` 外也查询 `Type::String`/`Type::Int` 等的 impl
  - 确保 `impl Printable for string { fn format(self): string { ... } }` 可被 trait bound 检查识别

  **Must NOT do**:
  - 不实现 `impl Trait for dyn`（动态类型）
  - 不修改 codegen 的 vtable 生成（内置类型方法分派属后续工作）
  - 不添加新的内置类型

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 单文件修改，逻辑清晰（添加类型映射 + 查询分支）
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (与 T7, T8, T9 并行——仅改 typechecker)
  - **Parallel Group**: Wave 2
  - **Blocks**: None
  - **Blocked By**: None

  **References**:
  - `crates/ruyic/src/typechecker/traits.rs:124-165` — `register_impl` 实现
  - `crates/ruyic/src/typechecker/traits.rs:173-181` — `implements` 查询函数
  - `crates/ruyic/src/typechecker/traits.rs:314-319` — `type_annotation_name` 辅助函数（需扩展）
  - `crates/ruyic/src/typechecker/types.rs` — `Type::String`, `Type::Int`, `Type::Float`, `Type::Bool` 定义

  **Acceptance Criteria**:
  - [ ] `cargo check -p ruyic` 无编译错误
  - [ ] `impl Printable for string { ... }` 通过 trait bound 检查
  - [ ] 单元测试验证内置类型 impl 可被泛型函数识别

  **QA Scenarios**:

  ```
  Scenario: impl Trait for built-in string type checks
    Tool: Bash
    Preconditions: 创建 test_impl_string.ry 含 `trait P { fn fmt(self): string; } impl P for string { fn fmt(self): string { return self; } } fn print_twice<T: P>(v: T) { ... }`
    Steps:
      1. ruyic test_impl_string.ry --check 2>&1
    Expected Result: 退出码 0（无 "trait not implemented" 错误）
    Failure Indicators: 包含 "does not implement trait" 或类似错误
    Evidence: .sisyphus/evidence/task-10-impl-string.txt
  ```

  **Commit**: YES (groups with G2)
  - Message: `feat(typechecker): support impl Trait for built-in types`
  - Files: `crates/ruyic/src/typechecker/traits.rs`

- [x] 11. **match 语句代码生成** (P1 — 最高实现风险)

  **What to do**:
  - **警告**：`patterns.rs` 全为空操作存根，需从头实现。这是本计划中工作量最大、风险最高的任务。
  - 在 `codegen/patterns.rs` 中实现 `PatternCompiler` 各方法：
    - `compile_primitive_match`：将 `match x { 1 => ..., 2 => ..., _ => ... }` 编译为 if-else 链或 LLVM `switch` 指令
    - `compile_bool_match`：`match flag { true => ..., false => ... }` → br 分支
    - `compile_string_match`：字符串比较链
    - `compile_nullable_match`：`match opt { Some(v) => ..., None => ... }` → null check + 解包
    - `compile_object_match` / `compile_array_match`：解构模式匹配
    - `generate_destructuring`：将模式绑定生成 `Expr::Member` 或局部变量赋值
  - 在 `codegen/stmt.rs` 的 `compile_stmt` 中添加 `Statement::Match` 分支，调用 `PatternCompiler`
  - 优先实现最小可用版本：仅支持 `match x { literal => expr, _ => expr }`（常量模式 + 通配符）
  - P0 交付：常量模式 + 通配符 + bool 模式
  - P1 交付：nullable 模式（`Some`/`None`）+ 解构模式（`{ field }`）

  **Must NOT do**:
  - 不在 codegen 层做穷尽性检查（typechecker 职责，v0.4 已完成）
  - 不实现 `match` with `if` guard（`match x { n if n > 0 => ... }`）
  - 不实现 or-patterns（`1 | 2 => ...`）
  - 不实现 `match` 表达式的类型推断（typechecker 已处理）

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 全新实现完整的模式匹配编译器，涉及多种模式类型的 IR 生成 + 解构，复杂度最高
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (与 T12 并行——不同文件)
  - **Parallel Group**: Wave 3
  - **Blocks**: None
  - **Blocked By**: Wave 2 完成（确保基础设施稳定）

  **References**:
  - `crates/ruyic/src/codegen/patterns.rs:1-440` — 当前存根 `PatternCompiler`（所有方法为空操作）
  - `crates/ruyic/src/codegen/stmt.rs:18-46` — `compile_stmt` 路由（需添加 `Statement::Match`）
  - `crates/ruyic/src/parser/ast.rs` — `Statement::Match`, `MatchArm`, `Pattern` 的 AST 定义
  - `crates/ruyic/src/typechecker/patterns.rs:21-59` — typechecker 的 `analyze_patterns()` 参考模式遍历方式

  **Acceptance Criteria**:
  - [ ] `cargo check -p ruyic` 无编译错误
  - [ ] 常量模式 `match x { 1 => "one", 2 => "two", _ => "other" }` 生成 if-else 链 IR
  - [ ] 通配符 `_` 生成 fallthrough 默认分支
  - [ ] `match result { Some(v) => v, None => 0 }` 生成 null check + 解包

  **QA Scenarios**:

  ```
  Scenario: match on int literal generates switch/if-else IR
    Tool: Bash
    Preconditions: 创建 test_match_int.ry 含 `match x { 1 => "one", 2 => "two", _ => "other" }`
    Steps:
      1. ruyic test_match_int.ry --emit-llvm -o /tmp/test_match.ll
      2. grep -c 'br' /tmp/test_match.ll
    Expected Result: 至少 3 个 `br` 指令（每个分支 + fallthrough）
    Evidence: .sisyphus/evidence/task-11-match-int.ll.txt

  Scenario: match on bool compiles
    Tool: Bash
    Preconditions: 创建 test_match_bool.ry 含 `match flag { true => 1, false => 0 }`
    Steps:
      1. ruyic test_match_bool.ry --check
    Expected Result: 退出码 0
    Evidence: .sisyphus/evidence/task-11-match-bool-check.txt

  Scenario: match with destructuring compiles
    Tool: Bash
    Preconditions: 创建 test_match_destructure.ry 含 `match opt { Some(v) => v, None => 0 }`
    Steps:
      1. ruyic test_match_destructure.ry --check
    Expected Result: 退出码 0
    Evidence: .sisyphus/evidence/task-11-match-destructure-check.txt
  ```

  **Commit**: YES (groups with G3)
  - Message: `feat(codegen): implement match statement codegen`
  - Files: `crates/ruyic/src/codegen/patterns.rs`, `crates/ruyic/src/codegen/stmt.rs`

- [x] 12. **线程本地 GC 堆** (P2)

  **What to do**:
  - 将 `GLOBAL_COLLECTOR: Lazy<Mutex<SendCollector>>` 改为每线程一个收集器实例
  - 使用 `thread_local!` + `RefCell<GenerationalCollector>` 为每个线程创建独立 GC 堆
  - 线程退出时自动清理该线程的 GC 堆
  - 更新 `ruyi_gc_alloc` / `ruyi_gc_collect` 等 C 导出函数使用当前线程的收集器
  - 对于跨线程共享的对象（如 spawn 的 future），实现简单的指针迁移（从源线程堆拷贝到目标线程堆）
  - **若实现复杂度过高**：本任务可降级为在 v0.5.0 中实现，仅在此做基础调研

  **Must NOT do**:
  - 不实现并发 GC（stop-the-world 即可）
  - 不实现跨线程写屏障优化
  - 不移除已有全局收集器（保留为默认）

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: GC 架构变更，涉及线程安全 + 指针迁移，复杂度高，P2 优先级
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES (与 T11 并行——不同模块)
  - **Parallel Group**: Wave 3
  - **Blocks**: None
  - **Blocked By**: Wave 2 完成

  **References**:
  - `crates/ruyi_runtime/src/gc_exports.rs:22-23` — 当前 `GLOBAL_COLLECTOR` 单例
  - `crates/ruyi_runtime/src/gc_exports.rs:37-47` — `ruyi_gc_alloc` / `ruyi_gc_collect` C 导出
  - `crates/ruyi_runtime/src/gc/generational.rs:21-29` — `GenerationalCollector` 结构体
  - `crates/ruyi_runtime/src/async_runtime.rs` — 线程调度器参考

  **Acceptance Criteria**:
  - [ ] `cargo check -p ruyi_runtime --no-default-features` 无编译错误
  - [ ] 每线程有独立的 `GenerationalCollector` 实例
  - [ ] `ruyi_gc_alloc` 使用当前线程的收集器

  **QA Scenarios**:

  ```
  Scenario: thread-local GC compiles
    Tool: Bash
    Preconditions: None
    Steps:
      1. cargo check -p ruyi_runtime --no-default-features 2>&1
    Expected Result: 退出码 0
    Evidence: .sisyphus/evidence/task-12-check.txt

  Scenario: thread-local GC uses thread_local not Mutex
    Tool: Bash
    Preconditions: None
    Steps:
      1. grep -n 'thread_local' crates/ruyi_runtime/src/gc_exports.rs
    Expected Result: 匹配到 `thread_local!` 声明
    Evidence: .sisyphus/evidence/task-12-thread-local.txt
  ```

  **Commit**: YES (groups with G3)
  - Message: `feat(runtime): implement thread-local GC heaps`
  - Files: `crates/ruyi_runtime/src/gc_exports.rs`, `crates/ruyi_runtime/src/gc/generational.rs`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. Verify each "Must Have" has implementation. Search for forbidden patterns. Check evidence files exist.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo check --workspace` + `cargo clippy --workspace`. Review all changed files for AI slop patterns.
  Output: `Build [PASS/FAIL] | Clippy [N warnings] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Execute EVERY QA scenario from EVERY task. Test cross-task integration. Test edge cases.
  Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  Verify 1:1 — everything in spec was built, nothing beyond spec. Check "Must NOT do" compliance.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | VERDICT`

---

## Commit Strategy

### Commit Groups
- **G0** (Prerequisite): `chore: create dev/v0.4.1 branch and bump version`
- **G1** (Wave 1 complete): `feat(codegen): add for loops, break/continue, optional chaining, template literals, BigInt`
- **G2** (Wave 2 complete): `feat(runtime): wire async await, exception landing pads, GC roots; feat(typechecker): impl Trait for built-ins`
- **G3** (Wave 3 complete): `feat(codegen): add match statement; feat(runtime): thread-local GC heaps`
- **G4** (Final verification): `chore: v0.4.1 cleanup verification and version bump`

### Version Bump Checklist
- [ ] `Cargo.toml`: workspace `version = "0.4.1"`
- [ ] `crates/ruyic/src/main.rs`: `#[command(version = "0.4.1")]`
- [ ] `docs/roadmap.md` + `docs/roadmap-zh.md`: mark v0.2-v0.4 as complete

---

## Success Criteria

### Verification Commands
```bash
# Regression check
cargo check --workspace && echo "PASS: no compile errors"

# Runtime tests
cargo test -p ruyi_runtime --no-default-features && echo "PASS: runtime tests"

# Integration check
cargo check -p ruyic && echo "PASS: compiler check"

# Typecheck-only validation for all example files
for f in examples/*.ry; do ruyic "$f" --check && echo "PASS: $f" || echo "FAIL: $f"; done
```

### Final Checklist
- [ ] 所有 11 项 "Must Have" 有实现
- [ ] 所有 "Must NOT Have" 未被违反
- [ ] `cargo check --workspace` 零错误
- [ ] Agent QA 场景全部通过
- [ ] 版本号更新至 v0.4.1
- [ ] 路线图更新完成
