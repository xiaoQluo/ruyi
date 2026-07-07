# 遗留问题修复计划

## TL;DR

> **Quick Summary**: 完成 Ruyi 编译器五阶段修复后的 9 项遗留工作，涵盖测试修复、埋桩封顶、架构重写，分四波次推进。

> **Deliverables**:
> - 3 个预存测试全部通过
> - Tuple 类型完整 LLVM struct 代码生成
> - CodegenContext 可变状态字段全面封装
> - Object 模式穷尽性递归矩阵检查
> - gc_roots 生命周期审计与加固
> - TypeEnvironment 接入 Codegen（受限范围）
> - 老年代 GC 标记-压缩（性能优化）
> - 模式矩阵有限增强（Object/Constructor 递归）
> - CI 增加 `-- --ignored` 运行 codegen 集成测试

> **Estimated Effort**: Large (4 waves, ~16 tasks)
> **Parallel Execution**: YES — 4 waves, max 5 parallel
> **Critical Path**: Wave 1 → Wave 2 → Wave 3 → Wave 4

---

## Context

### Original Request
完成 Ruyi 编译器五阶段修复后的全部 9 项遗留问题。

### Interview Summary
**Key Discussions**:
- 范围：全部 9 项，按影响面和依赖关系排优先级
- 测试：TDD 流程（RED → GREEN → REFACTOR）
- 审查：标准模式（Metis + 自审）
- Issue 7 范围：有限增强，仅 Object/Constructor 递归矩阵
- Issue 8 目标：性能优化，老年代标记-压缩
- Issue 6 深度：仅可变状态字段（8-10 个）
- Codegen 测试：继续 `#[ignore]`，CI 加 `-- --ignored`

**Research Findings**:
- `docs/spec.md` §5：泛型语法使用尖括号 `Array<Int>`，Issue 3 是真正的解析器 bug
- `gc_roots` push/pop 在 `generator.rs:617/676` 成对存在，但异常路径需审计
- Codegen 测试标记 `#[ignore]` 因需 LLVM，CI 不运行

### Metis Review
**Identified Gaps** (addressed):
- gc_roots 无 pop：验证为误报（push/pop 平衡）
- Issue 3 可能是无效测试：确认为真实解析器 bug（spec.md 确认尖括号语法）
- Issues 7/8 被严重低估：重新分类为架构里程碑，缩小范围
- Issue 6 需增加 gc_roots 异常路径审计：纳入 Wave 2

---

## Work Objectives

### Core Objective
消灭全部 9 项遗留问题，将代码库从"功能可用"提升至"生产级质量"。

### Concrete Deliverables
- `cargo test -p ruyic --lib`：127/127 全部通过（当前 124/127）
- `cargo test -p ruyic --test codegen -- --ignored`：新增测试 CI 可运行
- Tuple 类型生成 LLVM struct（非 `i8*` 占位符）
- CodegenContext 可变字段改为私有 + 访问方法
- 老年代 GC 使用滑动压缩算法
- Object 模式穷尽性支持递归字段值检查

### Definition of Done
- [ ] `cargo test --workspace` 零失败
- [ ] `cargo check --workspace` 零警告
- [ ] 全部 9 项 Issue 的验收标准满足

### Must Have
- 3 个预存测试通过
- Tuple LLVM struct 代码生成
- CodegenContext 可变字段封装（无公开可变状态）
- GC 标记-压缩不引入内存泄漏
- 所有修改不引入新测试失败

### Must NOT Have (Guardrails)
- 不碰 spec.md 语言规范（以 spec 为准修实现）
- 不重写整个 CodegenContext API（仅封装可变字段）
- 不全量实现 SML/NJ 模式矩阵（仅 Object/Constructor 递归增强）
- 不重写整个 GC（仅修改 `collect_full` 中老年代路径）
- 不改变对外暴露的 CLI 接口

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed.

### Test Decision
- **Infrastructure exists**: YES (`cargo test`)
- **Automated tests**: TDD — RED (failing test) → GREEN (minimal impl) → REFACTOR
- **Framework**: `cargo test` (Rust built-in, 已有 per-module 测试)
- **If TDD**: 每个 Issue 先复现失败 → 最小修复 → 运行全量回归

### QA Policy
每个任务包含 Agent-Executed QA Scenarios。证据保存至 `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`。

- **Backend/Compiler**: Bash (`cargo test`, `cargo check`, `cargo clippy`)
- **API/Module**: Bash (`cargo test --lib <module>`)

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately — 修复预存测试，3 项并行):
├── Task 1: 修复 test_from_annotation_generic [quick]
├── Task 2: 修复 test_bool_patterns_with_wildcard [quick]
└── Task 3: 修复 test_check_match_statement [quick]

Wave 2 (After Wave 1 — 埋桩封顶，4 项并行):
├── Task 4: Tuple LLVM struct 代码生成 [deep]
├── Task 5: CodegenContext 可变字段封装 [deep]
├── Task 6: Object 模式递归穷尽性检查 [deep]
└── Task 7: gc_roots 生命周期审计与加固 [unspecified-high]

Wave 3 (After Wave 2 — TypeEnvironment 接入):
├── Task 8: TypeEnvironment variables 作用域接入 Codegen [deep]
└── Task 9: CI 增加 codegen 集成测试 [quick]

Wave 4 (After Wave 3 — 架构重写，2 项并行):
├── Task 10: 老年代 GC 标记-压缩 [deep]
└── Task 11: 模式矩阵有限增强（Object/Constructor） [deep]

Wave FINAL (After ALL tasks — 4 项并行审查):
├── Task F1: Plan Compliance Audit (oracle)
├── Task F2: Code Quality Review (unspecified-high)
├── Task F3: Real Manual QA (unspecified-high)
└── Task F4: Scope Fidelity Check (deep)

Critical Path: Task 1 → Task 4 → Task 8 → Task 10 → F1-F4
Parallel Speedup: ~60% faster than sequential
Max Concurrent: 5 (Wave 2 + 3)

---

## TODOs

### Wave 1: 修复预存测试失败（3 项并行）

- [x] 1. 修复 `test_from_annotation_generic` — 泛型注解解析

  **What to do**:
  - 根据 `docs/spec.md` §5（尖括号泛型语法），修复 `Type::from_annotation` 将 `Array<Int>` 解析为 `Generic { base: "Array", args: [Int] }` 而非 `Array(Int)`
  - 在 `from_annotation` 中增加 `<` token 检测分支
  - 复现：`Type::from_annotation(&parse_type_annotation("Array<Int>"))` 应返回 `Type::Generic`
  - RED: 先写断言 `assert_eq!(parsed, Type::Generic { base: "Array", args: vec![Type::Int] })`
  - GREEN: 修复 `from_annotation` 中的解析逻辑
  - REFACTOR: 提取 `parse_generic_args` 为辅助函数

  **Must NOT do**:
  - 不修改 spec.md 中的语法定义
  - 不引入新的泛型解析路径（走现有 `from_annotation`）
  - 不改变 `Generic` 枚举变体的定义

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 单文件、单函数修复，逻辑清晰
  - **Skills**: [`test-driven-development`]
    - `test-driven-development`: RED-GREEN-REFACTOR 流程
  - **Skills Evaluated but Omitted**: None

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:
  - `docs/spec.md:1848-1862` — 泛型函数语法，确认 `Array<Int>` 尖括号规范
  - `crates/ruyic/src/typechecker/types.rs:620-680` — `from_annotation` 实现，当前缺失 `<` 分支
  - `crates/ruyic/src/typechecker/types.rs:655` — 失败断言行

  **Acceptance Criteria**:
  - [ ] `cargo test -p ruyic --lib typechecker::types::tests::test_from_annotation_generic` → PASS
  - [ ] `Array<Int>` 解析为 `Generic { base: "Array", args: [Int] }`
  - [ ] `Map<String, Int>` 解析为 `Generic { base: "Map", args: [String, Int] }`

  **QA Scenarios**:
  ```
  Scenario: 泛型注解解析正确
    Tool: Bash (cargo test)
    Preconditions: spec.md §5 确认尖括号语法
    Steps:
      1. 写测试：assert_eq!(from_annotation(&parse("Array<Int>")), Generic { base: "Array", args: [Int] })
      2. 运行 cargo test -p ruyic --lib typechecker::types::tests::test_from_annotation_generic
      3. 断言 FAIL（RED 阶段）
      4. 修改 from_annotation 添加 < 分支
      5. 运行测试断言 PASS（GREEN 阶段）
    Expected Result: test_from_annotation_generic 通过
    Failure Indicators: 测试仍然 panics at types.rs:655
    Evidence: .sisyphus/evidence/task-1-generic-parse.txt

  Scenario: 多参数泛型也正确
    Tool: Bash (cargo test)
    Steps:
      1. 追加测试：assert_eq!(from_annotation(&parse("Map<String, Int>")), Generic { base: "Map", args: [String, Int] })
      2. 运行测试断言 PASS
    Expected Result: 多参数泛型也正确解析
    Evidence: .sisyphus/evidence/task-1-generic-multi.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `fix(typechecker): parse generic type annotation with angle brackets`
  - Files: `crates/ruyic/src/typechecker/types.rs`

- [x] 2. 修复 `test_bool_patterns_with_wildcard` — 通配符冗余检测

  **What to do**:
  - 分析 `analyze_patterns` 中的 redundancy 逻辑：`Pattern::Identifier` 插入 `_` 导致下一臂 `Pattern::Wildcard` 的 `_` 被视为重复
  - `Identifier` 绑定的语义是"匹配任意值并绑定到变量名"，不是通配符"匹配任意值不绑定"
  - RED: 确认测试失败（`true` → `_` 序列不应标记 `_` 冗余，因为 `_` 覆盖了 `false` 未被 `true` 覆盖）
  - 但实际上当前逻辑：`true` 被插入了 `all_covered`，然后 `_` 被检查 → `_` 不在 `all_covered` 中 → 不应标记冗余
  - 问题可能在于 `Pattern::Identifier` 同时插入了 `_`，导致处理 `true` 后 `all_covered = {"true", "_"}`，然后 `_` 被检查时发现已存在

  **Must NOT do**:
  - 不改变 `PatternAnalysis` 的返回结构
  - 不破坏其他 pattern 测试（7 个现有通过的测试）

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 单函数逻辑修复，仅需调整 redundancy 检测条件
  - **Skills**: [`test-driven-development`]
    - `test-driven-development`: RED-GREEN-REFACTOR

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3)
  - **Blocks**: Task 6 (Object exhaustiveness enhancement)
  - **Blocked By**: None

  **References**:
  - `crates/ruyic/src/typechecker/patterns.rs:36-49` — `analyze_patterns` redundancy 检测循环
  - `crates/ruyic/src/typechecker/patterns.rs:69-73` — `Pattern::Identifier` 插入 `_` 的逻辑
  - `crates/ruyic/src/typechecker/patterns.rs:278-289` — `test_bool_patterns_with_wildcard` 测试

  **Acceptance Criteria**:
  - [ ] `cargo test -p ruyic --lib typechecker::patterns::tests::test_bool_patterns_with_wildcard` → PASS
  - [ ] `result.has_redundancy` 在 `true` → `_` 序列中应返回 `false`
  - [ ] `result.redundant_arm` 在无冗余时应为 `None`
  - [ ] 其余 7 个 pattern 测试全部通过

  **QA Scenarios**:
  ```
  Scenario: true 然后通配符不应冗余
    Tool: Bash (cargo test)
    Preconditions: 当前测试失败 at patterns.rs:288
    Steps:
      1. 运行 cargo test -p ruyic --lib typechecker::patterns::tests::test_bool_patterns_with_wildcard
      2. 观察：assertion failed: result.has_redundancy（期望 true 但实际 false）
      3. 分析：Identifier 插入 _ 导致后续 Wildcard 的 _ 被误判冗余
      4. 修改：Identifier 不插入 _（仅 Wildcard 插入 _）
      5. 更新测试期望：has_redundancy = false, redundant_arm = None
    Expected Result: 测试通过，redundancy 逻辑正确
    Failure Indicators: 其他 pattern 测试失败
    Evidence: .sisyphus/evidence/task-2-wildcard-redundancy.txt

  Scenario: 真正的冗余序列仍能检测
    Tool: Bash (cargo test)
    Steps:
      1. 构造：Wildcard → Literal(true) → 期望冗余 arm=1
      2. 构造：Identifier("x") → Identifier("y") → 期望冗余 arm=1（都匹配任意值）
      3. 运行测试断言 PASS
    Expected Result: 真冗余仍被检测
    Evidence: .sisyphus/evidence/task-2-real-redundancy.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `fix(typechecker): fix wildcard redundancy detection in pattern analysis`
  - Files: `crates/ruyic/src/typechecker/patterns.rs`

- [x] 3. 修复 `test_check_match_statement` — match 语句类型检查

  **What to do**:
  - RED: 运行 `test_check_match_statement`，观察错误输出
  - 分析 `check_program("match (1) { 1 => { } }")` 产生的类型错误
  - 可能原因：match 臂的返回类型推导为 `void`，但 match 表达式期望一个类型
  - 检查 `typechecker/checker.rs` 中 match 语句的类型检查逻辑
  - GREEN: 修复使其接受无返回值的 match 语句（或添加隐式 `void` 类型）
  - REFACTOR: 确保 match 臂类型检查逻辑清晰

  **Must NOT do**:
  - 不改变 match 表达式的语义（match 作为表达式仍需统一返回类型）
  - 不破坏其他 checker 测试

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 需理解 typechecker 逻辑但修复范围小
  - **Skills**: [`test-driven-development`, `systematic-debugging`]
    - `test-driven-development`: RED-GREEN-REFACTOR
    - `systematic-debugging`: 分析类型检查错误的根因

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:
  - `crates/ruyic/src/typechecker/checker.rs:171-176` — 失败的测试
  - `crates/ruyic/src/typechecker/checker.rs` — match 语句类型检查实现
  - `docs/spec.md` §4 — match 表达式语义（match 可作语句也可作表达式）

  **Acceptance Criteria**:
  - [ ] `cargo test -p ruyic --lib typechecker::checker::tests::test_check_match_statement` → PASS
  - [ ] `check_program("match (1) { 1 => { } }")` 返回 `!result.has_errors`
  - [ ] match 表达式（有返回值）仍然正常类型检查

  **QA Scenarios**:
  ```
  Scenario: match 语句无返回值应通过类型检查
    Tool: Bash (cargo test)
    Preconditions: 当前测试失败 at checker.rs:175
    Steps:
      1. 运行 cargo test -p ruyic --lib typechecker::checker::tests::test_check_match_statement
      2. 观察错误输出：? result.has_errors is true
      3. 分析根因：match 臂类型推导逻辑
      4. 修改类型检查器
      5. 运行测试断言 PASS
    Expected Result: 测试通过，has_errors = false
    Evidence: .sisyphus/evidence/task-3-match-check.txt

  Scenario: match 表达式有返回值仍正常工作
    Tool: Bash (cargo test)
    Steps:
      1. 追加测试：check_program("let x = match (1) { 1 => 42; _ => 0; };") has no errors
      2. 追加测试：check_program("let x: string = match (1) { 1 => 42; _ => 0; };") has type error
      3. 运行测试断言 PASS
    Expected Result: match 表达式类型推导正确
    Evidence: .sisyphus/evidence/task-3-match-expr.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `fix(typechecker): allow void match statement arms in type checker`
  - Files: `crates/ruyic/src/typechecker/checker.rs`

### Wave 2: 埋桩封顶（4 项并行）

- [x] 4. Tuple 类型生成 LLVM struct

  **What to do**:
  - 当前 `Type::Tuple(Vec<Type>)` 在 codegen 中退化为 `i8*` 指针占位符
  - RED: 添加 codegen 测试：编译包含 tuple 的 `.ry` 文件，验证输出 LLVM IR 含 struct 定义
  - 实现：在 `codegen/types.rs` 或 `codegen/generator.rs` 中增加 Tuple → LLVM struct 映射
  - 使用 inkwell 创建带命名字段（f0, f1, …）的 struct 类型
  - GREEN: 运行 codegen 测试（需 LLVM 环境）
  - REFACTOR: 提取 `codegen_tuple_type` 函数

  **Must NOT do**:
  - 不改变 `Type::Tuple` 枚举定义
  - 不引入 `<N>` 长度的编译时常量泛型（保持动态大小 Tuple）

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 需理解 LLVM IR struct 布局和 inkwell API
  - **Skills**: [`test-driven-development`]
    - `test-driven-development`: RED-GREEN-REFACTOR

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 6, 7)
  - **Blocks**: None
  - **Blocked By**: Task 1 (依赖 Tuple 类型定义稳定)

  **References**:
  - `crates/ruyic/src/typechecker/types.rs:37-38` — `Type::Tuple(Vec<Type>)` 定义
  - `crates/ruyic/src/codegen/types.rs:44-46` — Tuple → `i8*` 映射（需替换）
  - `crates/ruyic/src/codegen/generator.rs` — CodegenContext，LLVM 模块构建
  - Inkwell docs: `struct_type` API — `context.struct_type(&[field_types], packed=false)`

  **Acceptance Criteria**:
  - [ ] `Type::Tuple(vec![Type::Int, Type::String])` 生成 `{ i64, i8* }` LLVM struct
  - [ ] 元组字面量 `(1, "hello")` 编译为 `insertvalue` + `insertvalue` 指令序列
  - [ ] 元组字段访问 `t.0`、`t.1` 编译为 `extractvalue` 指令
  - [ ] 新增 codegen 集成测试（`#[ignore]`）验证端到端

  **QA Scenarios**:
  ```
  Scenario: 元组编译为 LLVM struct
    Tool: Bash (ruyic --emit-llvm)
    Preconditions: LLVM 14 环境
    Steps:
      1. 创建 test_tuple.ry：let t = (1, "hello");
      2. 编译：ruyic test_tuple.ry --emit-llvm
      3. Assert IR 输出包含 %tuple = type { i64, ptr }
      4. Assert IR 输出包含 insertvalue 构造元组
    Expected Result: IR 含元组 struct 定义和构造/访问指令
    Evidence: .sisyphus/evidence/task-4-tuple-llvm.txt

  Scenario: 元组字段访问正确
    Tool: Bash (ruyic --emit-llvm)
    Steps:
      1. 创建 test_tuple_access.ry：fn f() { let t = (1, 2); return t.0; }
      2. 编译：ruyic test_tuple_access.ry --emit-llvm
      3. Assert IR 输出包含 extractvalue 指令提取字段 0
    Expected Result: 字段访问编译为 extractvalue
    Evidence: .sisyphus/evidence/task-4-tuple-access.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `feat(codegen): emit LLVM struct for Tuple type`
  - Files: `crates/ruyic/src/codegen/types.rs`, `crates/ruyic/src/codegen/generator.rs`

- [x] 5. CodegenContext 可变字段封装

  **What to do**:
  - 识别 `CodegenContext` 中 8-10 个频繁修改的 `pub` 字段：
    `builder`, `variables`, `current_function`, `current_module`, `loop_stack`, `scopes`, `label_counter`, `gc_roots`, `string_literals`, `current_break_target`, `current_continue_target`
  - 为每个字段添加 getter/setter 方法或 struct 访问方法
  - 迁移所有直接字段访问为方法调用（`ctx.builder` → `ctx.builder()`）
  - RED: 编译期先确认字段可改为私有（`cargo check` 报错误点）
  - GREEN: 逐步添加方法并迁移调用点
  - REFACTOR: 检查是否有字段可以合成更高级别的 API

  **Must NOT do**:
  - 不封装所有 22 个 pub 字段（仅可变状态字段）
  - 不改变公共 API 语义（方法行为与字段直访一致）
  - 不重写 CodegenContext 的架构

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 涉及多个文件的 API 迁移
  - **Skills**: [`test-driven-development`]
    - `test-driven-development`: 每次迁移后验证编译和测试

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 6, 7)
  - **Blocks**: Task 8 (TypeEnvironment→Codegen)
  - **Blocked By**: None

  **References**:
  - `crates/ruyic/src/codegen/generator.rs:120-170` — CodegenContext 结构体定义
  - `crates/ruyic/src/codegen/stmt.rs` — 主要调用点（for/while/loop）
  - `crates/ruyic/src/codegen/expr.rs` — ctx.variables 大量使用
  - `crates/ruyic/src/codegen/decl.rs` — ctx.builder, ctx.current_function

  **Acceptance Criteria**:
  - [ ] 8-10 个可变状态字段改为 `pub(crate)` 或私有
  - [ ] 所有直接字段访问迁移为方法调用
  - [ ] `cargo check --workspace` 通过
  - [ ] `cargo test -p ruyic --lib` 维持 124+/127 通过
  - [ ] `cargo clippy` 无新增警告

  **QA Scenarios**:
  ```
  Scenario: 封装后编译通过
    Tool: Bash (cargo check)
    Preconditions: 字段改为私有
    Steps:
      1. cargo check --workspace
      2. Assert: 编译通过，零错误
    Expected Result: cargo check 通过
    Evidence: .sisyphus/evidence/task-5-check.txt

  Scenario: 封装后测试不受影响
    Tool: Bash (cargo test)
    Steps:
      1. cargo test -p ruyic --lib
      2. Assert: 测试数与修改前一致（≥124 通过）
    Expected Result: 无新增测试失败
    Evidence: .sisyphus/evidence/task-5-test.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `refactor(codegen): encapsulate mutable fields in CodegenContext`
  - Files: `crates/ruyic/src/codegen/generator.rs`, `crates/ruyic/src/codegen/stmt.rs`, `crates/ruyic/src/codegen/expr.rs`, `crates/ruyic/src/codegen/decl.rs`, `crates/ruyic/src/codegen/async_codegen.rs`

- [x] 6. Object 模式递归穷尽性检查

  **What to do**:
  - 当前 `find_missing_cases` 对 `Type::Object` 已修改为递归检查字段值（上一步改动）
  - 增强：对 Constructor 类型（用户自定义 class）也递归检查字段模式
  - 添加对深层组合的覆盖：如 `{ status: 200, body: ... }` 中 body 字段的模式是否穷尽
  - RED: 添加测试：Object 深层字段值模式未穷尽应报告缺失
  - GREEN: 在 `find_missing_cases` 的 Object 分支中完善递归逻辑
  - REFACTOR: 提取 `covered_prefix` 解析为公共函数

  **Must NOT do**:
  - 不引入完整的 PatternMatrix（留给 Task 11）
  - 不改变 `analyze_patterns` 的公共 API

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 递归模式分析需理解类型系统和 AST 结构
  - **Skills**: [`test-driven-development`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 5, 7)
  - **Blocks**: Task 11 (模式矩阵增强)
  - **Blocked By**: Task 2 (wildcard redundancy fix)

  **References**:
  - `crates/ruyic/src/typechecker/patterns.rs:197-220` — Object 分支当前实现（已部分改进）
  - `crates/ruyic/src/typechecker/patterns.rs:150-233` — `find_missing_cases` 完整逻辑
  - `crates/ruyic/src/typechecker/types.rs` — `Type::Object(Vec<ObjectField>)` 定义

  **Acceptance Criteria**:
  - [ ] `ObjectField { name: "status", ty: Type::Int }` 的 `find_missing_cases` 检查字段值模式
  - [ ] 当字段值类型为 `Type::Bool` 且只覆盖 `true` 时，报告缺失 `false`
  - [ ] 现有 pattern 测试全部通过

  **QA Scenarios**:
  ```
  Scenario: Object 字段值模式未穷尽
    Tool: Bash (cargo test)
    Steps:
      1. 构造 Object 类型字段 {status: Bool, body: String}
      2. 模式: {status: true, body: _}
      3. find_missing_cases 应返回 ["status: false"]（status 字段的 Bool 值未穷尽）
    Expected Result: 递归检测字段值模式，报告深层缺失
    Evidence: .sisyphus/evidence/task-6-object-recursive.txt

  Scenario: 全通配符 Object 仍判为穷尽
    Tool: Bash (cargo test)
    Steps:
      1. 构造 Object 模式: {status: _, body: _}
      2. find_missing_cases 应返回 []（所有字段值都通配）
    Expected Result: 通配字段不报告缺失
    Evidence: .sisyphus/evidence/task-6-object-wildcard.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `feat(typechecker): recursive exhaustiveness check for Object field patterns`
  - Files: `crates/ruyic/src/typechecker/patterns.rs`

- [x] 7. gc_roots 生命周期审计与加固

  **What to do**:
  - 审计 `push_gc_root_scope`/`pop_gc_root_scope` 的所有调用点
  - 检查异常路径（`throw`、提前 `return`）是否可能跳过 `pop_gc_root_scope`
  - 检查异步代码路径：`async_codegen.rs` 中的 gc_roots 操作
  - 如果发现泄漏路径，使用 RAII 风格 guard（`Drop` 自动弹出）加固
  - RED: 无法直接测试（GC 泄漏是静默的），改为审计报告
  - GREEN: 如有泄漏，加 `GcRootGuard` 或 `defer!` 宏确保弹出

  **Must NOT do**:
  - 不改变 GC 核心算法
  - 不引入新的 unsafe 代码

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: 静态分析 + 代码审计，无自动化测试
  - **Skills**: [`systematic-debugging`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 5, 6)
  - **Blocks**: None
  - **Blocked By**: Task 5 (CodegenContext 封装可能改 gc_roots 访问方式)

  **References**:
  - `crates/ruyic/src/codegen/generator.rs:131-160` — `push_gc_root_scope`/`pop_gc_root_scope` 实现
  - `crates/ruyic/src/codegen/generator.rs:617,676` — 主调用点（push/pop 成对）
  - `crates/ruyic/src/codegen/decl.rs` — `push_gc_root_scope` 调用
  - `crates/ruyic/src/codegen/async_codegen.rs` — 异步代码路径

  **Acceptance Criteria**:
  - [ ] 审计报告列出所有 push/pop 调用点及配对状态
  - [ ] 如有不配对，添加 RAII guard 或 `defer` 模式修复
  - [ ] `cargo test -p ruyi_runtime --lib` 70/70 通过
  - [ ] 审计结论写入注释（`// SAFETY: pop_gc_root_scope guaranteed by GcRootGuard Drop`）

  **QA Scenarios**:
  ```
  Scenario: gc_roots push/pop 配对审计
    Tool: Bash (grep + 人工分析)
    Steps:
      1. grep push_gc_root_scope 列出所有调用点
      2. grep pop_gc_root_scope 列出所有调用点
      3. 逐对匹配：每个 push 是否有对应的 pop 到达
      4. 检查异常路径：throw 后是否有 finally/cleanup 执行 pop
    Expected Result: 审计报告产出，配对状态明确
    Evidence: .sisyphus/evidence/task-7-gc-roots-audit.md

  Scenario: 如有泄漏修复后运行时测试正常
    Tool: Bash (cargo test)
    Steps:
      1. 如有修复，运行 cargo test -p ruyi_runtime --lib
      2. Assert: 70/70 通过
    Expected Result: 运行时测试不受影响
    Evidence: .sisyphus/evidence/task-7-runtime-test.txt
  ```

  **Commit**: YES (groups with Wave 2)
  - Message: `fix(codegen): audit and harden gc_roots scope push/pop lifecycle`
  - Files: `crates/ruyic/src/codegen/generator.rs`

### Wave 3: TypeEnvironment 接入 + CI 增强

- [x] 8. TypeEnvironment variables 作用域接入 Codegen

  **What to do**:
  - 当前 `CodeGenerator::generate()` 不接受 `TypeEnvironment`，从 annotation 重新推导类型
  - 受限范围：仅将 `TypeEnvironment` 中 `variables` 作用域信息传入 CodegenContext
  - 在 CodegenContext 中新增 `type_environment: Option<&TypeEnvironment>` 字段
  - 修改 `lookup_variable` 方法优先使用 TypeEnvironment（fallback 到 annotation 推导）
  - RED: 添加测试：有类型标注的变量在 codegen 中使用推断类型而非 annotation
  - GREEN: 实现 TyEnv 接入路径
  - REFACTOR: 确保 fallback 路径正常工作

  **Must NOT do**:
  - 不重构整个 CodegenContext API（留给后续）
  - 不修改 `TypeEnvironment` 的公开接口
  - 不改变无类型标注变量的行为（保持 dyn 推导）

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 跨模块（typechecker→codegen）的数据流修改
  - **Skills**: [`test-driven-development`]

  **Parallelization**:
  - **Can Run In Parallel**: NO（依赖 Task 5 封装的 CodegenContext）
  - **Parallel Group**: Wave 3 (sequential with Task 9)
  - **Blocks**: None
  - **Blocked By**: Task 5 (CodegenContext 封装完成)

  **References**:
  - `crates/ruyic/src/typechecker/environment.rs` — `TypeEnvironment` 及 `Scope` 定义
  - `crates/ruyic/src/codegen/generator.rs:208-220` — `CodeGenerator::generate()` 签名
  - `crates/ruyic/src/driver.rs` — 编译管线中 tyenv 的创建和使用

  **Acceptance Criteria**:
  - [ ] `CodeGenerator::generate()` 接受 `Option<&TypeEnvironment>`
  - [ ] 有 TypeEnvironment 时，变量类型查找优先使用 TyEnv
  - [ ] `cargo test -p ruyic --lib` 维持 ≥124 通过
  - [ ] 无 TyEnv 时（测试路径），fallback 到 annotation 推导

  **QA Scenarios**:
  ```
  Scenario: TyEnv 接入后变量类型正确
    Tool: Bash (cargo test)
    Steps:
      1. 创建程序：let x: int = 42; x + 1（类型应推断为 int）
      2. 在 codegen 中验证 x 的类型从 TyEnv 获得（非从 annotation 重新推导）
      3. 运行完整测试套件
    Expected Result: 类型推导正确，codegen 使用 TyEnv 结果
    Evidence: .sisyphus/evidence/task-8-tyenv-wire.txt

  Scenario: 无 TyEnv 时 fallback 正常
    Tool: Bash (cargo test)
    Steps:
      1. 在无 TyEnv 时运行 codegen（如测试路径）
      2. Assert: fallback 到 annotation 推导，不 panic
    Expected Result: fallback 路径正常工作
    Evidence: .sisyphus/evidence/task-8-tyenv-fallback.txt
  ```

  **Commit**: YES (groups with Wave 3)
  - Message: `feat(codegen): wire TypeEnvironment variable scope into CodegenContext`
  - Files: `crates/ruyic/src/codegen/generator.rs`, `crates/ruyic/src/driver.rs`

- [x] 9. CI 增加 codegen 集成测试

  **What to do**:
  - 修改 `.github/workflows/ci.yml`，在 test job 后新增 `codegen-test` job
  - 该 job 需 LLVM 环境，添加 `-- --ignored` 运行 codegen 集成测试
  - 使用 `if: github.event_name != 'pull_request'` 或设置 continue-on-error（LLVM 可能不可用）
  - RED: CI 文件修改无法本地测试，验证 YAML 语法
  - GREEN: 推送后 CI 自动运行

  **Must NOT do**:
  - 不改变现有 test job 的行为
  - 不移除 `#[ignore]` 标记

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 单文件 CI 配置修改
  - **Skills**: None

  **Parallelization**:
  - **Can Run In Parallel**: NO（依赖 Task 4 的 codegen 测试）
  - **Parallel Group**: Wave 3 (sequential)
  - **Blocks**: None
  - **Blocked By**: Task 4 (Tuple codegen 测试需要此 CI 运行)

  **References**:
  - `.github/workflows/ci.yml` — 当前 CI 配置
  - `crates/ruyic/tests/codegen.rs` — codegen 集成测试（`#[ignore]`）

  **Acceptance Criteria**:
  - [ ] CI 配置包含 `codegen-test` job
  - [ ] `codegen-test` job 运行 `cargo test -p ruyic --test codegen -- --ignored`
  - [ ] YAML 语法有效（可用 `yamllint` 验证）

  **QA Scenarios**:
  ```
  Scenario: CI YAML 语法有效
    Tool: Bash (yamllint 或 python -c "import yaml")
    Steps:
      1. python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"
      2. Assert: 无解析错误
    Expected Result: YAML 语法正确
    Evidence: .sisyphus/evidence/task-9-ci-yaml.txt
  ```

  **Commit**: YES (groups with Wave 3)
  - Message: `ci: add codegen integration test job with LLVM`
  - Files: `.github/workflows/ci.yml`

### Wave 4: 架构重写（2 项并行）

- [ ] 10. 老年代 GC 标记-压缩

  **What to do**:
  - 当前 `collect_full` 对新旧代都进行复制回收（`ruyi_alloc` 每个存活对象）
  - 性能优化目标：老年代改用标记-压缩（sliding compaction），减少碎片
  - 实现步骤：
    1. 在 `OldGeneration` 中改为使用连续堆内存块（`Vec<u8>` 或 mmap 区域）
    2. 标记阶段（复用现有 bitmap 标记）
    3. 计算转发地址（sliding：存活对象从低地址连续排列）
    4. 更新引用：遍历所有根和已标记对象，将指针更新为转发地址
    5. 压缩阶段：memmove 对象到新位置，更新 allocation pointer
  - RED: 添加 GC 测试：分配大量小对象，触发 `collect_full`，验证零碎片
  - GREEN: 实现标记-压缩
  - REFACTOR: 提取 `compact_old_generation` 为独立函数

  **Must NOT do**:
  - 不改变年轻代 GC（仍然复制）
  - 不改变 GC 公共 API（`allocate`、`collect` 签名不变）
  - 不引入新的 unsafe 代码量超过现有水平

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 核心内存管理算法，需深入理解 GC 和 unsafe Rust
  - **Skills**: [`test-driven-development`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Task 11)
  - **Blocks**: None
  - **Blocked By**: Task 7 (gc_roots 审计完成)

  **References**:
  - `crates/ruyi_runtime/src/gc/generational.rs` — `OldGeneration` 定义和 `collect_full` 实现
  - `crates/ruyi_runtime/src/gc/bitmap.rs` — 标记 bitmap
  - `crates/ruyi_runtime/src/gc/header.rs` — `GcObjectHeader` 定义
  - Jones & Lins "Garbage Collection" §5.3 — 标记-压缩算法参考

  **Acceptance Criteria**:
  - [ ] `OldGeneration` 支持标记-压缩路径
  - [ ] `collect_full` 调用 `compact_old_generation` 替代复制
  - [ ] 标记-压缩后无悬挂指针（valgrind / Miri clean）
  - [ ] `cargo test -p ruyi_runtime --lib` 70/70 通过
  - [ ] 分配 10000 个小对象后触发 GC，内存使用量不增长

  **QA Scenarios**:
  ```
  Scenario: 标记-压缩后内存正常
    Tool: Bash (cargo test)
    Steps:
      1. 编写测试：分配大量对象，触发 collect_full
      2. 验证老年代使用标记-压缩路径
      3. 验证所有对象可达且正确
    Expected Result: 标记-压缩完成，无悬挂指针
    Evidence: .sisyphus/evidence/task-10-gc-compact.txt

  Scenario: 运行时测试全部通过
    Tool: Bash (cargo test -p ruyi_runtime --lib)
    Steps:
      1. 运行全量运行时测试
      2. Assert: 70/70 通过
    Expected Result: 无回归
    Evidence: .sisyphus/evidence/task-10-runtime-test.txt
  ```

  **Commit**: YES (groups with Wave 4)
  - Message: `feat(gc): implement mark-compact for old generation`
  - Files: `crates/ruyi_runtime/src/gc/generational.rs`

- [x] 11. 模式矩阵有限增强（Object/Constructor 递归）

  **What to do**:
  - 在 Task 6 的基础上，增强 Constructor（用户 class）模式的穷尽性检查
  - 实现 `PatternMatrix` 基础结构，但仅限于 Object/Constructor 类型
  - 不支持 ADT 的全部构造子矩阵（那需要完整 SML/NJ 算法）
  - RED: 添加测试：用户 class 的字段模式未穷尽
  - GREEN: 实现 Constructor 的递归字段检查（类似 Object 但处理 class 字段）
  - REFACTOR: 保持代码可扩展到完整矩阵算法

  **Must NOT do**:
  - 不实现完整的 SML/NJ 模式矩阵（`Specialization`、`DefaultMatrix`、`IsUseful`）
  - 不改变 `PatternAnalysis` 公共 API
  - 不影响非 Object/Constructor 类型的穷尽性检查

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 类型感知的模式分析
  - **Skills**: [`test-driven-development`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Task 10)
  - **Blocks**: None
  - **Blocked By**: Task 6 (Object 递归穷尽性)

  **References**:
  - `crates/ruyic/src/typechecker/patterns.rs` — 模式分析核心
  - `crates/ruyic/src/typechecker/types.rs` — `Type::Named`（class 类型）
  - `crates/ruyic/src/parser/ast.rs` — Constructor 模式定义

  **Acceptance Criteria**:
  - [ ] Constructor 模式（如 `Point { x, y }`）递归检查字段值模式
  - [ ] Named 类型匹配字段定义穷尽性
  - [ ] 现有 pattern 测试全部通过
  - [ ] `cargo test -p ruyic --lib` 无新增失败

  **QA Scenarios**:
  ```
  Scenario: Constructor 字段模式穷尽
    Tool: Bash (cargo test)
    Steps:
      1. 定义 class Point { x: int, y: int }
      2. 模式: Point { x: 0, y: _ }
      3. find_missing_cases 应报告 ["x: <other int value>"]（x 字段的 int 值未穷尽）
    Expected Result: Constructor 递归检查字段值
    Evidence: .sisyphus/evidence/task-11-constructor.txt

  Scenario: Named 类型字段定义匹配
    Tool: Bash (cargo test)
    Steps:
      1. Named 类型有 3 个字段，模式只匹配 2 个
      2. Assert: 报告缺失第 3 个字段
    Expected Result: 字段数不匹配被检测
    Evidence: .sisyphus/evidence/task-11-named-missing.txt
  ```

  **Commit**: YES (groups with Wave 4)
  - Message: `feat(typechecker): recursive exhaustiveness for Constructor patterns`
  - Files: `crates/ruyic/src/typechecker/patterns.rs`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists. For each "Must NOT Have": search codebase for forbidden patterns. Check evidence files in `.sisyphus/evidence/`.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo check --workspace` + `cargo clippy --workspace` + `cargo fmt --check`. Review changed files for: `as any`/`@ts-ignore`, empty catches, commented-out code, unused imports. Check AI slop.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Format [PASS/FAIL] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Run `cargo test --workspace -- --nocapture`. Execute EVERY QA scenario from EVERY task. Verify evidence files exist.
  Output: `Tests [N/N pass] | Scenarios [N/N pass] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff. Verify 1:1 — everything in spec was built, nothing beyond spec was built. Check "Must NOT do" compliance.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | VERDICT`

---

## Commit Strategy

- **Wave 1**: `fix(typechecker): resolve 3 pre-existing test failures` — checker.rs, patterns.rs, types.rs
- **Wave 2**: `feat(codegen,typechecker): tuple struct codegen + context encapsulation + pattern exhaustiveness` — generator.rs, types.rs, patterns.rs
- **Wave 3**: `feat(codegen): wire TypeEnvironment into Codegen` — generator.rs, driver.rs
- **Wave 4**: `feat(gc,typechecker): mark-compact GC + pattern matrix enhancement` — generational.rs, patterns.rs

---

## Success Criteria

### Verification Commands
```bash
cargo test --workspace                    # Expected: all pass (127/127 ruyic + 70/70 runtime)
cargo check --workspace                   # Expected: zero warnings
cargo clippy --workspace                  # Expected: zero warnings
cargo test -p ruyic --test codegen -- --ignored  # Expected: codegen tests pass
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All tests pass (zero failures)
- [ ] CI updated with codegen integration tests
