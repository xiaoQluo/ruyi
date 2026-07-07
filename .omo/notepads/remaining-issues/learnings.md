

## 2026-06-26: 实现 Tuple 类型的 LLVM struct 代码生成

### 变更
1. **`typechecker/types.rs`**:
   - 在 `Type` 枚举中新增 `Tuple(Vec<Type>)` 变体
   - 更新 `from_annotation`：将 `TypeAnnotation::Tuple` 正确映射为 `Type::Tuple`（之前退化为 `Type::Array`）
   - 更新 `Display`：实现 `(T1, T2, ...)` 格式输出
   - 更新 `is_subtype_of`：Tuple 按元素逐个协变比较
   - 更新 `least_upper_bound`：Tuple 按元素逐个计算 lub

2. **`typechecker/inference.rs`**:
   - 修改 `Expr::Sequence` 推断逻辑：当序列包含多个元素时，推断为 `Type::Tuple`（之前只返回最后一个元素的类型）
   - 修改 `synthesize_member_access`：当对象为 `Type::Tuple` 且属性名为数字索引时，返回对应位置的字段类型

3. **`codegen/types.rs`**:
   - 在 `ruyi_type_to_llvm` 中添加 `Type::Tuple` 分支，使用 `context.struct_type(&field_types, false)` 生成 LLVM struct 类型

4. **`codegen/expr.rs`**:
   - 新增 `compile_tuple_literal` 函数：为每个元素生成代码，使用 `insertvalue` 指令逐个插入到 struct 中
   - 新增 `compile_tuple_field_access` 函数：使用 `extractvalue` 指令按索引提取字段值
   - 在 `compile_expr` 中添加 `Expr::Sequence` 匹配，路由到 `compile_tuple_literal`
   - 在 `compile_member_access` 中检测 Tuple 类型，路由到 `compile_tuple_field_access`

5. **`typechecker/generics.rs`**:
   - 在 `mangle_type` 中添加 `Type::Tuple` 分支

6. **`tests/codegen.rs`**:
   - 新增 3 个 `#[ignore]` 标记的 codegen 集成测试：
     - `codegen_tuple_literal_and_access`
     - `codegen_tuple_mixed_types`
     - `codegen_tuple_field_arithmetic`

### 编译修复（附带）
工作目录中 `generator.rs` 的 `CodegenContext` 字段已被前序波次改为私有，但 `stmt.rs`/`traits.rs`/`expr.rs` 等尚未同步更新。为最小化波及范围，将相关字段改为 `pub(crate)`，同时保留已有的 getter 方法。另修复了 `stmt.rs` 中 `push_loop` 的调用方式（从元组参数改为展开参数）。

### 验证
- `cargo check --workspace` → 通过
- `cargo test -p ruyic --lib` → **129/129 通过**

### Key Insight
Tuple 在 Ruyi AST 中通过 `Expr::Sequence` 表示（因为 `(1, "hello")` 被解析为逗号分隔的序列）。在 typechecker 中，多元素 `Sequence` 推断为 `Type::Tuple`；在 codegen 中，`Sequence` 被编译为 LLVM struct。字段访问 `t.0` 在 AST 中通过 `Expr::Member` 表示，codegen 通过 `extractvalue` 实现。

## 2026-06-26: 封装 CodegenContext 可变状态字段

### 变更
1. **`codegen/generator.rs`**:
   - 将 8 个频繁修改的 `pub` 字段改为私有：`builder`, `variables`, `current_function`, `loop_stack`, `try_stack`, `current_return_type`, `expected_expr_type`, `allow_partial_codegen`
   - 新增 22 个 getter/setter 方法：
     - `builder()` / `builder_mut()`
     - `variables()` / `variables_mut()` / `lookup_variable()` / `define_variable()` / `remove_variable()`
     - `current_function()` / `set_current_function()`
     - `push_loop()` / `pop_loop()` / `current_loop()`
     - `push_try()` / `pop_try()` / `current_try()` / `try_stack_is_empty()`
     - `current_return_type()` / `set_current_return_type()`
     - `expected_expr_type()` / `set_expected_expr_type()`
     - `allow_partial_codegen()` / `set_allow_partial_codegen()`
   - 将 `define_variable` 的签名设计为接受元组 `(PointerValue, Type)`，以便与原有 `variables.insert(name, (ptr, ty))` 调用一一对应，降低迁移复杂度

2. **`codegen/stmt.rs`** / **`codegen/expr.rs`** / **`codegen/patterns.rs`** / **`codegen/decl.rs`** / **`codegen/async_codegen.rs`** / **`codegen/traits.rs`** / **`codegen/arc_ops.rs`**:
   - 将所有直接字段访问迁移为方法调用（共涉及 500+ 处引用）
   - 主要迁移模式：
     - `ctx.builder.` → `ctx.builder().`
     - `&ctx.builder` → `ctx.builder()`
     - `ctx.current_function = X` → `ctx.set_current_function(X)`
     - `ctx.current_function.ok_or(...)` → `ctx.current_function().ok_or(...)`
     - `ctx.variables.get(name)` → `ctx.lookup_variable(name)`
     - `ctx.variables.insert(name, (ptr, ty))` → `ctx.define_variable(name, (ptr, ty))`
     - `ctx.loop_stack.push(...)` → `ctx.push_loop(...)`
     - `ctx.try_stack.last()` → `ctx.current_try()`

### 验证
- `cargo check --workspace` → 通过
- `cargo test -p ruyic --lib` → **129/129 通过**
- `cargo clippy --workspace` → 无新增警告（所有 clippy error/warning 均为 `ruyi_runtime` 中已存在的问题）

### Key Insight
- 使用 `replaceAll` 进行批量字符串替换时，必须注意替换顺序：先处理赋值语句（如 `ctx.current_function = ` → `ctx.set_current_function(`），再处理读取访问（如 `ctx.current_function.ok_or` → `ctx.current_function().ok_or`），否则赋值语句会被错误地替换为 `ctx.current_function() = ...`
- 对于 `ctx.builder` 的多行链式调用（`ctx.builder\n    .build_...`），需要分两步替换：先替换行尾的 `ctx.builder\n`，再替换行首的 `.builder\n`，否则会出现漏匹配

## 2026-06-26: 将 TypeEnvironment 接入 CodegenContext

### 变更
1. **`codegen/generator.rs`**:
   - 为 `CodegenContext` 添加第三个生命周期参数 `'env`，并新增 `type_environment: Option<&'env TypeEnvironment>` 字段
   - 修改 `new` 方法签名，接受 `type_environment` 参数
   - 修改 `lookup_variable` 返回类型从 `Option<&(PointerValue, Type)>` 改为 `Option<(PointerValue, Type)>`，内部优先从 `type_environment` 查找类型，fallback 到本地 `variables` 中存储的类型
   - 新增 `generate_with_env` 方法，接受可选的 `TypeEnvironment`；`generate` 和 `generate_with_tracker` 均转发到该方法（传 `None`）
   - 同步更新 `GcRootScopeGuard` 的 3 生命周期参数

2. **`codegen/expr.rs`** / **`codegen/stmt.rs`**:
   - 适配 `lookup_variable` 返回值类型的变化：去掉 `*ptr` 解引用和 `.cloned()` 调用
   - 批量替换 `CodegenContext<'ctx, '_>` → `CodegenContext<'ctx, '_, '_>`（共涉及 100+ 处）

3. **`driver.rs`**:
   - 在 `compile_program` 中，将 `type_result.env` 和 `type_result.tracker` 通过 `generate_with_env` 传递给 codegen

### 验证
- `cargo check --workspace` → 通过
- `cargo test -p ruyic --lib` → **129/129 通过**

### Key Insight
- 当 `CodegenContext` 需要引入第三个生命周期参数时，虽然 struct 定义和 impl 块需要显式修改，但函数签名中的 `CodegenContext<'ctx, '_>` 只需增加一个 `'_` 占位符即可让编译器自动推断，可用 `sed` 批量替换
- `lookup_variable` 从返回引用改为返回值，是因为需要在返回前动态合并 `type_environment` 的类型和本地 `variables` 的 PointerValue，无法在栈上构造一个可返回引用的临时值
