# Tasks: Class Instance Member Access Codegen

## File Structure

| File | Responsibility |
|------|---------------|
| `crates/ruyic/src/codegen/expr.rs` (Modify) | 核心改动：重构 `compile_simple_member_access`、`compile_optional_member_access`、`compile_member_access` bracket 路径、`compile_assignment` Member 路径 |
| `crates/ruyic/tests/codegen.rs` (Modify) | 去掉 3 测试 `#[ignore]`，清理 TODO 注释 |

## Interfaces

**新增辅助函数（内部，不导出）**：
- `resolve_class_from_type(obj_ty: &Type) -> Option<String>` — 从 `Type::Named`/`Type::Array`/`Type::Generic`/`Type::Nullable` 提取 class name
- `class_field_access(ctx, obj_ptr, class_name, field_name) -> Result<(PointerValue, Type)>` — 双索引 GEP + load 字段值

**跨 Batch 接口**：无（全部在 `expr.rs` 内，单文件改动）

---

## Batch 1: 提取共享辅助函数

**依赖**：无

### Task 1.1: 提取 `resolve_class_from_type`

- **文件**：`crates/ruyic/src/codegen/expr.rs`
- **TDD Phase 1 (RED)**：无独立单元测试（通过集成测试验证）
- **TDD Phase 2 (GREEN)**：在 `compile_simple_member_access` 之前添加：
  ```rust
  fn resolve_class_from_type(obj_ty: &Type) -> Option<String> {
      match obj_ty {
          Type::Named(n, _) => Some(n.clone()),
          Type::Array(_) => Some("Array".to_string()),
          Type::Generic { base, .. } => Some(base.clone()),
          Type::Nullable(inner) => resolve_class_from_type(inner),
          _ => None,
      }
  }
  ```
- **TDD Phase 3 (REFACTOR)**：在 `compile_optional_member_access` 中将 lines 954-965 的类型匹配替换为调用此函数
- **TDD Phase 4 (VERIFY)**：`cargo check -p ruyic` 通过
- **TDD Phase 5 (REGRESSION)**：`cargo test -p ruyic --test codegen -- codegen_arithmetic --exact` 通过（确保现有 field 访问不受影响）
- **接口**：
  - Consumes: `&Type`
  - Produces: `Option<String>`

### Task 1.2: 提取 `class_field_access`

- **文件**：`crates/ruyic/src/codegen/expr.rs`
- **TDD Phase 1 (RED)**：无独立单元测试
- **TDD Phase 2 (GREEN)**：添加辅助函数，从 `compile_optional_member_access` 的 lines 999-1022 提取核心逻辑：
  ```rust
  fn class_field_access<'ctx>(
      ctx: &mut CodegenContext<'ctx, '_, '_>,
      obj_ptr: PointerValue<'ctx>,
      class_name: &str,
      field_name: &str,
  ) -> Result<(PointerValue<'ctx>, Type), String> {
      let fields = ctx.class_fields.get(class_name)
          .ok_or_else(|| format!("Unknown class: {}", class_name))?;
      let field_ty = fields.iter()
          .find(|(n, _)| n == field_name)
          .map(|(_, ty)| ty.clone())
          .ok_or_else(|| format!("Unknown field: {} in class {}", field_name, class_name))?;
      let struct_type = ctx.class_struct_types.get(class_name)
          .ok_or_else(|| format!("No struct type for class: {}", class_name))?;
      let struct_ptr = ctx.builder().build_pointer_cast(
          obj_ptr, struct_type.ptr_type(Default::default()),
          &format!("{}_cast", class_name),
      );
      let field_index = fields.iter().position(|(n, _)| n == field_name).unwrap();
      let i32_ty = ctx.context.i32_type();
      let field_ptr = unsafe {
          ctx.builder().build_gep(struct_ptr,
              &[i32_ty.const_int(0, false), i32_ty.const_int(field_index as u64, false)],
              &format!("{}_ptr", field_name))
      };
      Ok((field_ptr, field_ty))
  }
  ```
- **TDD Phase 3 (REFACTOR)**：`compile_optional_member_access` 的 lines 999-1022 替换为调用此函数
- **TDD Phase 4 (VERIFY)**：`cargo check -p ruyic` 通过
- **TDD Phase 5 (REGRESSION)**：同上 Task 1.1 的回归测试
- **接口**：
  - Consumes: `ctx`, `obj_ptr: PointerValue`, `class_name: &str`, `field_name: &str`
  - Produces: `(PointerValue, Type)` — GEP 后的字段指针 + 字段类型

---

## Batch 2: 重构 `compile_simple_member_access` (field read)

**依赖**：Batch 1

### Task 2.1: 重构为 `compile_expr` + 类型分发

- **文件**：`crates/ruyic/src/codegen/expr.rs`
- **TDD Phase 1 (RED)**：运行 `codegen_class_creation` 和 `codegen_fixture_member_access`（当前有 `#[ignore]`，先手动验证报错）
- **TDD Phase 2 (GREEN)**：重写 `compile_simple_member_access`：
  1. `let obj_result = compile_expr(ctx, object)?;` — 获取 LLVM 值 + 类型
  2. `let obj_ptr = obj_result.value.into_pointer_value();` — 提取指针
  3. `let class_name = resolve_class_from_type(&obj_result.ty).ok_or_else(|| ...)?;` — 类型 → class 名
  4. `let (field_ptr, field_ty) = class_field_access(ctx, obj_ptr, &class_name, field_name)?;` — GEP → 字段指针
  5. Load 字段值（保留现有的 Float bitcast 处理 logic）
  6. 返回 `ExprResult`
- **TDD Phase 3 (REFACTOR)**：删除原有的 `Expr::Identifier` / `Expr::SelfExpr` match arm；Shared helper handles everything
- **TDD Phase 4 (VERIFY)**：`cargo check -p ruyic` 通过
- **TDD Phase 5 (REGRESSION)**：Typechecker 222 测试全绿。手工测试 `let p = Point.new(3,4); print(p.x);` 仍输出 `3`
- **接口**：
  - Consumes: `ctx`, `object: &Expr`, `field_name: &str`
  - Produces: `ExprResult`（loaded field value）

---

## Batch 3: 重构 `compile_assignment` Member 路径 (field write)

**依赖**：Batch 1

### Task 3.1: 支持任意表达式 LHS 的 field write

- **文件**：`crates/ruyic/src/codegen/expr.rs`，函数 `compile_assignment` lines 2617-2733
- **TDD Phase 1 (RED)**：运行 `test_new_class_8_fields`（当前 `#[ignore]`）
- **TDD Phase 2 (GREEN)**：替换 `object.as_ref()` match arm（lines 2625-2653）：
  1. `let obj_result = compile_expr(ctx, object)?;` — 获取 LLVM 值 + 类型
  2. `let obj_ptr = obj_result.value.into_pointer_value();` — 提取指针
  3. `let class_name = resolve_class_from_type(&obj_result.ty).ok_or_else(|| ...)?;` — 类型 → class 名
  4. `let (field_ptr, _) = class_field_access(ctx, obj_ptr, &class_name, &field_name)?;` — GEP → 字段指针
  5. `ctx.builder().build_store(field_ptr, right_result.value);` — store
- **TDD Phase 3 (REFACTOR)**：删除 Identifier/SelfExpr 分支，统一用上述路径；Object 类型的 write 路径保留（lines 2655-2691，逻辑不同，用平坦偏移）
- **TDD Phase 4 (VERIFY)**：`cargo check -p ruyic` 通过
- **TDD Phase 5 (REGRESSION)**：手工测试 `let w = Wide.new(); w.a = 1; print(w.a);` 输出 `1`

---

## Batch 4: Bracket 访问 class 实例

**依赖**：Batch 1

### Task 4.1: `["key"]` 对 class 实例走 GEP

- **文件**：`crates/ruyic/src/codegen/expr.rs`，函数 `compile_member_access`，`MemberProperty::Expr(key)` 路径 (lines 643-711)
- **TDD Phase 1 (RED)**：运行 `codegen_fixture_member_access`（当前 `#[ignore]`）
- **TDD Phase 2 (GREEN)**：在 line 696 的 `else` 分支之前插入 class 实例处理：
  ```rust
  // 在 Type::Object 检查之后、回退到 ruyi_obj_get 之前
  if let Type::Named(class_name, _) = &obj_result.ty {
      if let Expr::StringLiteral(key_str) = key_expr.as_ref() {
          let obj_ptr = value_to_i8_ptr(ctx, &obj_result.value)?;
          let (field_ptr, field_ty) = class_field_access(ctx, obj_ptr, class_name, key_str)?;
          let value = ctx.builder().build_load(field_ptr, key_str);
          return Ok(ExprResult::new(value, field_ty));
      }
  }
  ```
- **TDD Phase 3 (REFACTOR)**：保持 Array dictionary access 和 generic ruyi_obj_get fallback 不变
- **TDD Phase 4 (VERIFY)**：`cargo check -p ruyic` 通过
- **TDD Phase 5 (REGRESSION)**：手工测试 `let p = Point.new(3,4); print(p["x"]);` 输出 `3`

---

## Batch 5: 修复 `compile_optional_member_access` 的类型处理

**依赖**：Batch 1

### Task 5.1: Nullable 类型 unwrap 完善

- **文件**：`crates/ruyic/src/codegen/expr.rs`，函数 `compile_optional_member_access` (lines 946-1031)
- **TDD Phase 1 (RED)**：运行 member_access fixture（当前 `?.` 路径报 "not indexable" warnings）
- **TDD Phase 2 (GREEN)**：检查当前逻辑 — 已经通过 `Type::Nullable` match 处理 unwrap（lines 958-963）。验证 warnings 来自 typechecker 而非 codegen。如果 codegen 将 null 值表示为 `i8* null`（指针），`ptr_to_int` + `== 0` 的 null check 是正确的。
- **TDD Phase 3 (REFACTOR)**：使用 `resolve_class_from_type` 替换 lines 954-965 的类型匹配
- **TDD Phase 4 (VERIFY)**：`cargo check -p ruyic` 通过
- **TDD Phase 5 (REGRESSION)**：手工测试 `let maybe: Point? = Point.new(3,4); print(maybe?.x);` 输出 `3`

---

## Batch 6: 测试启用与最终验证

**依赖**：Batch 2, 3, 4, 5

### Task 6.1: 去 `#[ignore]` + 清理

- **文件**：`crates/ruyic/tests/codegen.rs`
- **操作**：
  1. `codegen_class_creation`（line 302）：去掉 `#[ignore]` 和 TODO 注释
  2. `test_new_class_8_fields`（line 769）：去掉 `#[ignore]` 和 TODO 注释
  3. `codegen_fixture_member_access`（line 416）：去掉 `#[ignore]` 和 TODO 注释
  4. 不修改测试体

### Task 6.2: 逐个运行验证

- **命令**：
  ```bash
  RUYI_BIN=target/debug/ruyic cargo test -p ruyic --test codegen -- codegen_class_creation --exact -- --test-threads=1
  RUYI_BIN=target/debug/ruyic cargo test -p ruyic --test codegen -- test_new_class_8_fields --exact -- --test-threads=1
  RUYI_BIN=target/debug/ruyic cargo test -p ruyic --test codegen -- codegen_fixture_member_access --exact -- --test-threads=1
  ```
- 预期：3/3 passed

### Task 6.3: 全量回归

- `cargo test -p ruyic --test typechecker` — 222/222 passed
- `cargo check --workspace` — 0 errors
- `cargo clippy -p ruyic` — 无新增错误
