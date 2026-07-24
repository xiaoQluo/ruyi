# Design: Class Instance Member Access Codegen

## Context

当前 class member access codegen 有两套并行实现：

1. **`compile_simple_member_access`**（`expr.rs:799`）— 处理 `.field` 非可选访问。只支持 `Expr::Identifier` 和 `Expr::SelfExpr`。通过 `ctx.variables.get(name)` 取变量指针，再用 `(field_index * 8)` 做平坦 GEP 偏移。

2. **`compile_optional_member_access`**（`expr.rs:946`）— 处理 `?.field` 安全访问。已使用 `compile_expr(ctx, object)?` 获取 LLVM 值，支持任意表达式。通过 `class_struct_types` 做类型化指针转换和双索引 GEP `[i32 0, i32 field_index]`，null check 分支处理完善。

3. **bracket access**（`expr.rs:643-711`）— `MemberProperty::Expr(key)` 路径。只处理 `Type::Array`（走 `__builtin_array_get`）和 `Type::Object`（走内联字段查找），对 `Type::Named` class 实例回退到 `ruyi_obj_get`（通用 object lookup）。

4. **field write** — `compile_assignment` 处理 `Expr::Member` LHS，复用了 `compile_simple_member_access` 的 Identifier 查找逻辑。

**约束**：`class_fields` HashMap（class_name → Vec<(field_name, Type)>）在 codegen 初始化时由 `compile_class` 填充。`class_struct_types` HashMap（class_name → LLVM StructType）同时填充。变量表 `ctx.variables` 在声明时填充 name → (alloca_ptr, Type)。

**关键缺陷**：函数① 的所有字段查找都从 `ctx.variables` 出发，这意味着 object 必须是变量名。方法链 `Point.new(3,4).format()` 中的 `.format()` 的 object 是 `Expr::Call`，不是 Identifier，直接走 `_ => return Err("Member access only supported on identifiers")`。

## Goals

1. `compile_simple_member_access` 支持任意表达式 object
2. `?.` 完整路径：非 null 返回字段值、null 返回 0
3. `["key"]` bracket 在 class 实例上走 GEP 字段查找
4. field write 支持任意表达式 LHS
5. 不破坏现有 Identifier 路径和 typechecker

## Decisions

### Decision 1: 统一使用 `compile_expr` + 类型分发 模式

**Choice**：将 `compile_simple_member_access` 的核心逻辑从 "Identifier lookup → GEP" 改为 "compile_expr → 类型匹配 → GEP"，复用 `compile_optional_member_access` 中已验证的类型化指针转换和双索引 GEP 模式。

**Rationale**：
- `compile_optional_member_access` 已经证明了这个模式可行（line 951: `compile_expr(ctx, object)?` → 类型匹配 → `class_struct_types` → pointer_cast → 双索引 GEP → load）
- 当前 Identifier 路径中 `(field_index * 8)` 的平坦 GEP 不安全 — 它假设所有字段紧密排列且各占 8 字节，不适用于 float/int 混合、父类字段等场景
- 统一后只需维护一套字段访问逻辑

**Alternatives considered**：
- **保持两套逻辑，只扩展 match arm**：为 `Expr::Call`、`Expr::Member` 等各加 arm → 代码膨胀，每加一种表达式都要写新 arm
- **在调用方（`compile_member_access`）做 Identifier 转换**：把非 Identifier 表达式先存到临时变量 → 改变了语义（变量名污染），且临时变量的生命周期管理复杂

### Decision 2: 双索引 GEP 替代平坦偏移

**Choice**：字段访问 GEP 使用 `[i32 0, i32 field_index]` 双索引（0=基指针解引用，field_index=结构体成员索引），通过 `class_struct_types` 获取 LLVM 结构体类型做 pointer cast。

**Rationale**：
- LLVM 结构体有正确的字段对齐和填充，`getelementptr` 双索引形式由 LLVM 自动处理对齐
- 当前平坦偏移 `(field_index * 8)` 假设所有字段都是 i64，对 float（f64 → 通过 bitcast 模拟）已是 workaround，对其他类型（bool、ptr）会错位
- `compile_optional_member_access` 已在使用此模式并正确工作

**Alternatives considered**：
- **保持平坦偏移，但用 `size_of` 计算实际偏移**：需要手动维护字段大小表，与 LLVM 的 DataLayout 耦合
- **用 extractvalue 代替 GEP**：只适用于值类型的结构体，不支持指针类型的 class 实例

### Decision 3: 保留 Identifier 路径作为快速通道

**Choice**：保留 `Expr::Identifier` 的变量查找路径，但改为统一走 "获取 LLVM 值 → struct cast → GEP" 而非 "变量指针 → 平坦 GEP"。非 Identifier 表达式走 `compile_expr`，但最终字段访问逻辑共享同一段代码。

**Rationale**：
- 对 Identifier 变量访问是热路径，直接变量查找比 `compile_expr` 少一次 match dispatch
- 但字段偏移计算应统一，避免两个路径产生不同结果
- 提取共享的 `do_class_field_access(ctx, obj_ptr, class_name, field_name)` 辅助函数

### Decision 4: Bracket 访问增加 class 实例路径

**Choice**：在 `compile_member_access` 的 `MemberProperty::Expr(key)` 路径中，当 object type 是 `Type::Named` 时，检查 key 是否为字符串字面量，若是则掉用 GEP 字段查找（与 `.field` 相同），否则回退到 `ruyi_obj_get`。

**Rationale**：
- `p["x"]` 语义上是按字段名访问，应生成高效的 GEP 而非 runtime dict lookup
- 若 key 是变量或表达式（非字面量），保持回退到 `ruyi_obj_get` 以保证正确性
- 不改动 Array/Object 的现有路径

## Risks And Trade-Offs

| 风险 | 缓解 |
|------|------|
| 改动 `compile_simple_member_access` 影响所有 field 访问 | 提取共享辅助函数，Identifier 路径复用同一逻辑，减少分支差异 |
| 双索引 GEP 改变 LLVM IR 结构 | 仅在 class 实例访问路径上使用；Array/Object 路径不受影响；运行 typechecker 全量测试验证无回归 |
| field write 路径可能有独立问题 | 先诊断 `compile_assignment` 中 Member LHS 的处理逻辑；若需改，follow same pattern |
| nullable 类型在 `?.` 路径的 null 值表示 | 当前 `null` 表示为 `i8* null`（指针），`?.` 中 `ptr_to_int` + `== 0` 的 null check 正确，不改动 |
