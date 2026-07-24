# Proposal: fix-batch-low-risk-defects

## Why

Version v0.5.9 的缺陷审查发现 5 个优先级排列的遗留问题。其中优先级 2-5 为低风险缺陷，互不依赖，可在单个 change 中批量修复：

1. **`allow_partial_codegen` 全局覆盖** — `driver.rs:576` 将标志设为 `true`（对所有编译生效），静默吞掉用户代码的 codegen 错误。应限定为 stdlib-only。
2. **3 个测试断言错误** — `pending-issues.md` 记录的遗留缺陷：`test_check_match_statement`（checker.rs）、`test_bool_patterns_with_wildcard`（patterns.rs 逻辑反转）、`test_from_annotation_generic`（types.rs 期望值过期）。
3. **12 条 codegen "not yet supported" 路径** — `codegen/expr.rs` 和 `codegen/decl.rs` 中有 12 处直接返回错误，阻断相关语言特性使用。
4. **路线图文档过时** — `roadmap.md`（滞后 5 版本）和 `roadmap-zh.md`（滞后 1 版本）未标记 v0.5.5-v0.5.9 已完成项。

## What Changes

### 1. allow_partial_codegen 限定 stdlib-only (`driver.rs`)

将 `generator.allow_partial_codegen = true` 从无条件赋值改为条件赋值：仅当编译涉及 stdlib 模块时才启用。由于当前 driver 无条件 auto-load stdlib，实现方式为在生成器创建时传递 stdlib 标识，或改为 `false` 并在发现 codegen 无法处理的 stdlib 符号时优雅降级。

### 2. 修复 3 个测试断言

| 测试 | 文件:行 | 修复 |
|------|---------|------|
| `test_check_match_statement` | typechecker/checker.rs:172 | 更新断言以匹配 v0.5 type checker 行为 |
| `test_bool_patterns_with_wildcard` | typechecker/patterns.rs:266 | `assert!(result.has_redundancy)` → `assert!(!result.has_redundancy)` |
| `test_from_annotation_generic` | typechecker/types.rs:652 | 期望值从 `Generic{base:"Array", args:[Int]}` → `Type::Array(Box::new(Type::Int))` |

### 3. 实施 12 条 codegen not-yet-supported 路径

全部位于 `crates/ruyic/src/codegen/expr.rs`（11 条）和 `crates/ruyic/src/codegen/decl.rs`（1 条）：

| # | 文件:行 | 功能 | 策略 |
|---|---------|------|------|
| 1 | expr.rs:377 | 匿名函数 | 编译为具名闭包（同现有箭头函数模式） |
| 2 | expr.rs:387 | 异步箭头函数 | 编译为具名异步闭包 |
| 3 | expr.rs:2236 | 嵌套成员访问 | 递归编译成员链 |
| 4 | expr.rs:2336 | 间接调用 | 通过函数指针间接调用 |
| 5 | expr.rs:2405 | Spread 参数（调用点 1） | 展开参数打包 |
| 6 | expr.rs:2496 | Spread 参数（调用点 2） | 同 #5 |
| 7 | expr.rs:2563 | 复合赋值（`+=`、`-=` 等） | 读-运算-写模式 |
| 8 | expr.rs:2621 | 复杂赋值（数组索引、嵌套等） | 扩展赋值目标处理 |
| 9 | expr.rs:2974 | 复杂 new 表达式 | 支持 `new expr(args)` 非标识符形式 |
| 10 | expr.rs:2995 | Spread 参数（构造器调用点） | 同 #5 |
| 11 | expr.rs:3044 | Spread 参数（super 构造器） | 同 #5 |
| 12 | decl.rs:75 | 复杂模式（let/const 绑定） | 支持解构赋值模式 |

### 4. 更新路线图文档

- `docs/roadmap.md`：标记 v0.5.5-v0.5.9 已完成项，更新 Current State Assessment，同步版本表
- `docs/roadmap-zh.md`：同 EN 路线图对齐更新

## Scope

### In Scope
- `crates/ruyic/src/driver.rs`：allow_partial_codegen 条件化
- `crates/ruyic/src/typechecker/checker.rs`：修复 test_check_match_statement
- `crates/ruyic/src/typechecker/patterns.rs`：修复 test_bool_patterns_with_wildcard
- `crates/ruyic/src/typechecker/types.rs`：修复 test_from_annotation_generic
- `crates/ruyic/src/codegen/expr.rs`：实施 11 条 not-yet-supported 路径
- `crates/ruyic/src/codegen/decl.rs`：实施 1 条 not-yet-supported 路径
- `docs/roadmap.md` + `docs/roadmap-zh.md`：更新至 v0.5.9
- 可能新增 codegen 集成测试覆盖新支持的路径

### Out of Scope
- 异常 unwinder（优先级 1）— 独立 Change A: `fix-exception-unwinder`
- Mutex 迁移到 parking_lot（优先级 6）
- CI/CD 建立（优先级 6）
- v0.2.0/v0.3.0 tag 补打（优先级 6）
- proptest/fuzzing（优先级 6）
- GC、async、新语言特性
- 空 `impl RenderSeverity {}` 清理（pending-issues #5，低优）
- 错误代码文档修正（pending-issues #6，低优）

## Impact

| 模块 | 影响程度 | 风险 |
|------|----------|------|
| Driver | 低 | allow_partial_codegen 条件化后，用户代码中此前被静默忽略的错误将暴露 |
| Typechecker 测试 | 极低 | 仅修改 3 个测试断言 |
| Codegen | 中 | 12 条新路径需要完整的类型安全和 LLVM IR 正确性验证 |
| 文档 | 极低 | 路线图纯文本更新 |

## Capabilities

- **新增能力**：匿名函数编译、异步箭头编译、嵌套成员访问、间接调用、spread 参数、复合赋值、复杂赋值、复杂 new 表达式、复杂模式绑定——全部 12 条路径从"不支持"变为"支持"
- **修复**：allow_partial_codegen 不再隐藏用户代码 codegen 错误
- **修复**：3 个类型检查器测试断言正确
- **文档**：路线图反映 v0.5.9 真实完成状态

## Success Criteria

1. `make check` 通过，零新增 clippy 警告
2. `cargo test -p ruyic --test typechecker` 中原先失败的 3 个测试全部通过
3. 12 条 codegen 路径全部移除 `"not yet supported"` 错误返回，改为功能实现或更精确的诊断
4. `allow_partial_codegen` 仅对 stdlib 编译为 `true`
5. 路线图文档中 v0.5.5-v0.5.9 已完成项标记 ✅
6. 无回归——现有通过测试全部保持通过
