# 变更提案：v0.5.9-class-codegen

## Why

class 实例上的 member access codegen（字段读写、方法调用、安全访问）当前只支持最简单的 case：object 必须是 `Expr::Identifier`（变量名）。一旦 object 是函数调用结果（`get_point().x`）、方法链（`Point.new(3,4).format()`）、或更复杂的表达式，`compile_simple_member_access` 就走不通。

这直接阻塞了 3 个 codegen integration 测试（`codegen_class_creation`、`test_new_class_8_fields`、`codegen_fixture_member_access`），其中 fixture 测试覆盖了 `.field`、`?.field`、`["key"]` 三种成员访问模式在 class 实例上的完整路径。

T9 stdlib typecheck 修复已使 stdlib 正确编译，但 class codegen 仍然是 Ruyi 写面向对象代码的最后一道坎。

## DP-1: 需求确认

- **问题**：class 实例上的 member access codegen 不支持非 `Identifier` 表达式（如方法返回值、函数调用结果），导致 3 个 codegen 测试 ignore。`compile_simple_member_access` 只处理 `Expr::Identifier`，阻断方法链、`?.` 安全访问、`["key"]` 索引访问。
- **范围**：重构 `compile_simple_member_access` → 通用 `compile_expr→GEP→load/store` 路径，覆盖 class 实例的全部成员操作：`.` / `?.` / `["key"]` / `.field = val` / `obj.method()` / 方法链。
- **非目标**：trait/impl/泛型 class codegen。
- **成功标准**：`codegen_class_creation`、`test_new_class_8_fields`、`codegen_fixture_member_access` 去 ignore 全部通过；typechecker 222 全绿；cargo check 通过。
- **方案**：A — 重构为核心通用路径。
- **拆分**：单个变更。
- **确认时间**：2026-07-16
- **确认**：用户确认无误

## What Changes

重构 `crates/ruyic/src/codegen/expr.rs` 中的 `compile_simple_member_access` 函数，使其不再依赖 object 必须是 `Expr::Identifier`，而是通过先 `compile_expr(object)` 获取 LLVM 值，再基于值的类型信息做 GEP 偏移加载/存储字段。

同时修复 `compile_optional_member_access` 中 nullable class 实例的 `?.` 路径，以及 `MemberProperty::Expr` 路径中对 class 实例的 `["key"]` bracket 访问。

**具体改动**：

1. **`compile_simple_member_access`**：移除 `Expr::Identifier` 限制，改为 `compile_expr(object)` → 从类型信息查 `class_fields` → GEP 偏移 → load/store
2. **`compile_optional_member_access`**：修复 nullable class 实例的 null check + 字段访问
3. **`MemberProperty::Expr(key)` 路径**：对 `Type::Named` class 实例，通过 bracket key 做字段查找而非回退到 `ruyi_obj_get`
4. **field write（`obj.field = val`）路径**：确保赋值 LHS 支持非 Identifier 表达式

## Scope

**In**：
- `.field` 读写：任意表达式 object（Identifier、Call、Member、etc）
- `?.field` 安全访问：nullable class 实例
- `["key"]` bracket 访问：class 实例按字符串 key 字段查找
- 方法调用：`obj.method()` 通过 `Expr::Call { callee: Member }` 路径
- 方法链：`Point.new(3,4).format()`
- 测试：3 个 `#[ignore]` 全部启用并通过

**Out**：
- trait/impl dispatch codegen
- 泛型 class monomorphization codegen
- class 继承/父类字段访问 codegen
- 非 class 类型的 member access 路径变更

## Impact

| 区域 | 影响 |
|------|------|
| `crates/ruyic/src/codegen/expr.rs` | 核心改动 |
| `crates/ruyic/tests/codegen.rs` | 3 测试去 `#[ignore]` |
| 其他 codegen 模块 | 无影响（接口不变） |
| typechecker | 无影响 |
| parser | 无影响 |

## Capabilities

- 字段读写完整路径（CAP1-002）
- 方法调用与方法链（CAP1-003）
- `?.` 安全访问在 class 实例上的支持（CAP8-003）
