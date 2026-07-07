# Proposal: Fix Codegen Gaps (3 Features)

## Why

当前编译器 parser/typechecker 已支持 BigInt 模式匹配、宏声明、类型别名三类语法,但 codegen 阶段在处理这 3 类声明时直接报错,导致 `examples/` 下 3 个文件无法编译。本变更补全这 3 个 codegen 缺口,并新增 BigInt 字面量比较的 runtime 支持,使 example 套件从 30/33 提升到 33/33。

## What Changes

### 1. BigInt 模式匹配 (patterns.rs:36)

**根因**(已确认): `compile_match_stmt` 的 match 路由把 `Type::BigInt` 错误地与 `Type::Int` 捆绑,送入 `compile_int_match`。`compile_int_match` 期望 i64 整数,但 BigInt 的 LLVM 表示是 `i8*` 指针,所以报错 `Int match requires integer scrutinee`。

**修复**: 从 `Type::Int | Type::BigInt` 分支移除 `BigInt`,让它落入默认的 `compile_generic_match` 路径。

### 2. BigInt 字面量 match (新增需求,需扩 scope 到 ruyi_runtime)

**根因**:
- 当前 `examples/bigint.ry` 的 match 用例仅覆盖通配符 `_`
- 用户要求新增字面量比较的测试代码
- `ruyi_runtime` 下**无** BigInt 等值比较函数(`ruyi_bigint_eq` 不存在)
- BigInt 内部表示为 i8* 指针,无法用 LLVM `icmp` 直接比较

**修复**:
- 在 `crates/ruyi_runtime/src/builtins.rs` 新增 `ruyi_bigint_eq(a: *mut i8, b: *mut i8) -> i8` 函数(placeholder 实现可接受)
- 在 `crates/ruyi_runtime/src/lib.rs` re-export
- 在 `codegen/patterns.rs` 的 `compile_generic_match` 路径中,处理 `Pattern::Literal(BigIntLiteral(_))` 时,生成对 `ruyi_bigint_eq` 的调用

**Scope 扩展**: 本变更从原计划的 1 个 crate(`ruyic`)扩展到 2 个 crate(`ruyic` + `ruyi_runtime`)。

### 3. 宏声明 codegen (codegen/decl.rs)

**根因**: `codegen_declaration` 的 match 缺少 `Declaration::Macro` 分支。

**修复**: 添加 `Declaration::Macro { .. } => Ok(None)` —— 宏声明是编译时抽象(宏展开器在 typechecker 之前已处理),不产生 LLVM IR。

### 4. 类型别名 codegen (codegen/decl.rs)

**根因**: `codegen_declaration` 的 match 缺少 `Declaration::TypeAlias` 分支。

**修复**: 添加 `Declaration::TypeAlias { .. } => Ok(None)` —— 类型别名是编译时抽象,不产生运行时代码。

## Scope

### In Scope
- `crates/ruyic/src/codegen/patterns.rs`: BigInt 路由修复 + 字面量 match codegen
- `crates/ruyic/src/codegen/decl.rs`: Macro / TypeAlias 跳过分支
- `crates/ruyi_runtime/src/builtins.rs`: 新增 `ruyi_bigint_eq` 函数
- `crates/ruyi_runtime/src/lib.rs`: re-export `ruyi_bigint_eq`
- `examples/bigint.ry`: 新增字面量 match 测试代码
- 验证 33/33 example 通过

### Out of Scope (Scope Fence)
- ❌ bigint 四则运算 codegen 优化
- ❌ BigInt 真实数值比较语义(本变更用 placeholder,真实库集成时再升级)
- ❌ 宏运行时展开(展开器在 typechecker 前已处理)
- ❌ 类型别名语义验证(typechecker 已处理)
- ❌ 14 个 WIP 既存测试失败(`Builtin` vs `Identifier` 不匹配,与本变更无关)

## Impact

| 影响面 | 评估 |
|--------|------|
| 编译产物 | 0 变化(跳过声明 = 不产生 IR);新增 1 个 runtime 函数 |
| 运行行为 | BigInt match 字面量分支走 `ruyi_bigint_eq`(placeholder) |
| 测试 | 3 个 example 从 FAIL → PASS;新增 1 个字面量 match 测试 |
| 性能 | 通用路径 + 1 个函数调用,BigInt 实际使用场景不敏感 |
| ABI | 新增 export 函数,不破坏现有 ABI |

## Capabilities

- `language-match`: 扩展 BigInt 类型在 match 表达式中的支持(含字面量)
- `language-macro`: 宏声明 codegen 路径补全
- `language-type-alias`: 类型别名 codegen 路径补全
- `runtime-bigint`: 新增 BigInt 运行时支持(等值比较)

## Acceptance

```bash
bash examples/run_examples.sh
→ Total: 33 | Passed: 33 | Failed: 0

cargo build --release
→ 零警告

cargo test -p ruyi_runtime --no-default-features --lib
→ 全部通过
```
