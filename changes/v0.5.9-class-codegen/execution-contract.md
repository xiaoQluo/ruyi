# 执行合同：v0.5.9-class-codegen

> **Change**: v0.5.9-class-codegen | **Branch**: 当前（后续创建） | **State**: dp_2_approved → bridging
> **Workflow**: full

## Intent Lock

- **变更名称**：v0.5.9-class-codegen
- **要解决的问题**：class 实例上的 member access codegen（字段读写、方法调用）当前只支持 `Expr::Identifier` 作为 object。方法链 `Point.new(3,4).format()`、`?.field` 安全访问、`["key"]` bracket 访问在 class 实例上全部 blocked。导致 3 个 codegen 测试 `#[ignore]`。
- **范围内**：重构 `compile_simple_member_access` 为通用 `compile_expr → 类型分发 → GEP → load/store` 路径。覆盖 `.field` 读写、`?.field` 安全访问、`["key"]` bracket 访问、方法调用链。提取 `resolve_class_from_type` 和 `class_field_access` 两个共享辅助函数。
- **范围外**：trait/impl dispatch codegen、泛型 class monomorphization codegen、class 继承/父类字段访问、非 class 类型的 member access 路径变更、parser/typechecker 改动。

## Approved Behavior

| REQ | 行为摘要 |
|-----|----------|
| MA-001 | `compile_simple_member_access` 接受任意表达式 object → `compile_expr` → 类型匹配 → 双索引 GEP → load |
| MA-002 | `compile_assignment` Member 路径接受任意表达式 LHS → `compile_expr` → GEP → store |
| MA-003 | 方法链 `Point.new(3,4).format()` 通过 `compile_call` 的 `Expr::Member` callee 路径正确路由 |
| MA-004 | `?.field` 安全访问 nullable class 实例：非 null → 字段值，null → 0 |
| MA-005 | `["key"]` bracket 在 class 实例上走 `class_field_access()` GEP 查找，非 ruyi_obj_get |
| MA-006 | 8 字段 Wide class 分配正确 struct size |
| MA-007 | Typechecker 222/222 全绿 |
| MA-008 | 3 测试去 `#[ignore]` 全部通过 |

### 关键场景

- **MA-001** `print(Point.new(3, 4).x)` → 输出 `3`
- **MA-002** `Wide.new().a = 1; print(w.a);` → 输出 `1`（注：test_new_class_8_fields 测试完整 8 字段读写）
- **MA-003** `print(Point.new(3, 4).format())` → 输出 `(3, 4)`
- **MA-004** `let maybe: Point? = Point.new(3,4); print(maybe?.x);` → `3`；`let null_p: Point? = null; print(null_p?.x);` → `0`
- **MA-005** `print(p["x"])` → 输出 `3`
- **MA-006** `class Wide { a:int; b:int; … h:int; fn new(){} }` → `Wide.new()` 分配 8×i64 struct

### 验收检查

1. `codegen_class_creation` → 输出 `(3, 4)`
2. `test_new_class_8_fields` → 输出 `1\n2\n3\n4\n5\n6\n7\n8`
3. `codegen_fixture_member_access` → 输出 `3\n4\n3\n3\n0`
4. `cargo test -p ruyic --test typechecker` → 222 passed
5. `cargo check --workspace` → 0 errors
6. 无新增 clippy errors

## Design Constraints

- **架构约束**：
  - 全部改动在 `crates/ruyic/src/codegen/expr.rs` 单文件内
  - 提取 2 个 `fn` 内部辅助函数（不导出）：`resolve_class_from_type`、`class_field_access`
  - 双索引 GEP `[i32 0, i32 field_index]` 替代平坦偏移 `(field_index * 8)`
  - Identifier 路径保留为快速通道但共享同一字段访问逻辑
- **接口约束**：无跨文件接口。辅助函数签名：
  - `fn resolve_class_from_type(obj_ty: &Type) -> Option<String>`
  - `fn class_field_access<'ctx>(ctx, obj_ptr, class_name, field_name) -> Result<(PointerValue<'ctx>, Type), String>`
- **不新增外部 crate**
- **保留所有既有 `/** ... */` Javadoc**

## Task Batches

### Batch 1: 提取共享辅助函数
- **文件**：`crates/ruyic/src/codegen/expr.rs`
- **产出**：`resolve_class_from_type()` + `class_field_access()` 两个函数
- **完成定义**：`cargo check -p ruyic` 通过 + 现有 Identifier field 访问不受影响

### Batch 2: 重构 `compile_simple_member_access`
- **依赖**：Batch 1
- **文件**：`crates/ruyic/src/codegen/expr.rs`
- **产出**：函数接受任意表达式 object
- **完成定义**：手工验证 `print(Point.new(3,4).x)` 输出 `3` + typechecker 222 全绿

### Batch 3: 重构 `compile_assignment` Member 路径
- **依赖**：Batch 1
- **文件**：`crates/ruyic/src/codegen/expr.rs`
- **产出**：field write 接受任意表达式 LHS
- **完成定义**：手工验证 `Wide.new().a = 1` compile + typechecker 222 全绿

### Batch 4: Bracket 访问 class 实例
- **依赖**：Batch 1
- **文件**：`crates/ruyic/src/codegen/expr.rs`
- **产出**：`p["field"]` 对 class 实例走 GEP
- **完成定义**：手工验证 `print(p["x"])` 输出 `3` + typechecker 222 全绿

### Batch 5: 验证 `?.` 安全访问
- **依赖**：Batch 1
- **文件**：`crates/ruyic/src/codegen/expr.rs`
- **产出**：使用 `resolve_class_from_type` 替换内联类型匹配
- **完成定义**：手工验证 `print(maybe?.x)` 输出 `3` + `print(null_p?.x)` 输出 `0`

### Batch 6: 测试启用 + 全量回归
- **依赖**：Batch 2, 3, 4, 5
- **文件**：`crates/ruyic/tests/codegen.rs`
- **产出**：3 测试去 `#[ignore]` + 全部通过
- **完成定义**：3/3 passed + typechecker 222/222 + cargo check 0 errors

## Review Gates

| Gate | 检查点 |
|------|--------|
| Batch 1-5 每步 | `cargo check -p ruyic` 通过；typechecker 222 无回归 |
| Batch 6 | 3 测试逐个通过；全量 `cargo check --workspace` |
| 最终 | clippy 零新增 errors |

## Escalation Rules

- 若 Batch 2 重构后代码量超过 100 行新增 → 拆分辅助函数
- 若 `?.` 路径的 null check 逻辑与预期不符 → 不改动 null 表示（`i8* null`），只改类型匹配
- 若 `compile_assignment` 的 Object write 路径与 class write 路径冲突 → 提取独立 `compile_class_field_write` 函数
- 若上述 3 项均无法解决 → 退回 DP-2 重新评估设计决策

## Approval (DP-3)

本合同总结了 `proposal.md` + `specs/class-member-access-codegen.md` + `design.md` + `tasks.md` 四个规划制品的核心内容。

- **转发规则**：实施必须遵循 6 Batch 顺序（Batch 2-5 可并行于 Batch 1 后）。每 Batch 过完检查点再进下一 Batch。
- **模糊点**：无。所有 8 需求均有 WHEN/THEN 场景，所有任务均有五阶段 TDD 步骤。
- **未映射需求**：无。MA-001~008 全部对应到 Batch 和验收检查。
