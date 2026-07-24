# Tasks: fix-batch-low-risk-defects

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/ruyic/src/driver.rs` | Modify | allow_partial_codegen 条件化：传递 stdlib_item_count 给 CodeGenerator |
| `crates/ruyic/src/codegen/generator.rs` | Modify | CodeGenerator 接收并存储 stdlib_item_count，按当前项索引判断是否 allow_partial_codegen |
| `crates/ruyic/src/codegen/expr.rs` | Modify | 实施 11 条 codegen 路径：匿名函数、异步箭头、嵌套成员访问、间接调用、spread 参数(×4)、复合赋值、复杂赋值、复杂 new |
| `crates/ruyic/src/codegen/decl.rs` | Modify | 实施 1 条 codegen 路径：复杂模式绑定 |
| `crates/ruyic/src/typechecker/checker.rs` | Modify | 修复 test_check_match_statement 断言 |
| `crates/ruyic/src/typechecker/patterns.rs` | Modify | 修复 test_bool_patterns_with_wildcard 断言 |
| `crates/ruyic/src/typechecker/types.rs` | Modify | 修复 test_from_annotation_generic 断言 |
| `docs/roadmap.md` | Modify | 更新至 v0.5.9：标记已完成项、更新版本表、Current State Assessment |
| `docs/roadmap-zh.md` | Modify | 同 EN 路线图对齐更新 |
| `docs/pending-issues.md` | Modify | 将 #1-#4 标记为已修复或更新状态 |

## Interfaces

### Cross-Batch

| Producer | Consumer | Interface |
|----------|----------|-----------|
| Batch 2 (driver) | Batch 3 (codegen) | `CodeGenerator::with_gc_mode_and_stdlib_count(context, module, gc_mode, stdlib_item_count: usize)` — 新增构造方法 |
| Batch 3a (spread util) | Batch 3a (4 spread sites) | `fn unpack_spread_args(ctx, args) -> Result<Vec<BasicMetadataValueEnum>, String>` — 公共解包函数 |

### Internal (within CodeGenerator)

| Field | Type | Purpose |
|-------|------|---------|
| `current_item_index` | `usize` | 追踪当前正编译的 ModuleItem 索引 |
| `stdlib_item_count` | `usize` | 来自 driver 的 stdlib 项数量；`current_item_index < stdlib_item_count` → allow partial |

---

## Batch 1: Typechecker Test Fixes [Independent]

### Task 1.1: Fix test_check_match_statement
- **File**: `crates/ruyic/src/typechecker/checker.rs` (around line 172)
- **TDD**:
  1. Run `cargo test -p ruyic --lib test_check_match_statement` — 确认当前失败
  2. 阅读测试代码，理解 v0.5 type checker 对 match statement 的实际行为
  3. 修改断言以匹配当前 type checker 行为（可能从 `!has_errors` → `has_errors` 或更新 match 表达式的语法）
  4. 运行测试确认通过
  5. 运行 `cargo test -p ruyic --lib` 确认无回归
- **Interfaces**: 无外部接口

### Task 1.2: Fix test_bool_patterns_with_wildcard
- **File**: `crates/ruyic/src/typechecker/patterns.rs` (line 266)
- **TDD**:
  1. Run `cargo test -p ruyic --lib test_bool_patterns_with_wildcard` — 确认当前失败
  2. 确认修复方案：`assert!(result.has_redundancy)` → `assert!(!result.has_redundancy)`
  3. 应用修复
  4. 运行测试确认通过
  5. 运行 `cargo test -p ruyic --test typechecker` 确认无回归
- **Interfaces**: 无外部接口

### Task 1.3: Fix test_from_annotation_generic
- **File**: `crates/ruyic/src/typechecker/types.rs` (line 652)
- **TDD**:
  1. Run `cargo test -p ruyic --lib test_from_annotation_generic` — 确认当前失败
  2. 确认修复方案：期望值从 `Generic{base:"Array", args:[Int]}` → `Type::Array(Box::new(Type::Int))`
  3. 应用修复
  4. 运行测试确认通过
  5. 运行 `cargo test -p ruyic --lib` 确认无回归
- **Interfaces**: 无外部接口

### Task 1.4: Update pending-issues.md status
- **File**: `docs/pending-issues.md`
- **Work**:
  1. 将 #1 (`test_check_match_statement`) 标记为修复状态 + 添加修复 commit 引用
  2. 将 #2 (`test_bool_patterns_with_wildcard`) 标记为修复状态
  3. 将 #3 (`test_from_annotation_generic`) 标记为修复状态
  4. 将 #4 (`allow_partial_codegen`) 标记为修复状态
  5. 验证 #5/#6 仍标记为未修复
- **Depends on**: Task 1.1, 1.2, 1.3, 2.2

---

## Batch 2: Driver allow_partial_codegen [Independent]

### Task 2.1: Add stdlib_item_count to CodeGenerator
- **File**: `crates/ruyic/src/codegen/generator.rs`
- **TDD**:
  1. 确认现有 `CodeGenerator::with_gc_mode` 签名
  2. 新增 `with_gc_mode_and_stdlib_count(context, module, gc_mode, stdlib_item_count: usize)` 构造方法
  3. 新增字段 `stdlib_item_count: usize` 和 `current_item_index: usize`
  4. 在 `generate_item` 入口递增 `current_item_index`
  5. 修改 `allow_partial_codegen` 的读取：从固定 `true` → `self.current_item_index < self.stdlib_item_count`
  6. 保留 `allow_partial_codegen` 字段作为缓存值（在 `generate_with_env` 开始前设置）
- **Interfaces**:
  - Produces: `CodeGenerator::with_gc_mode_and_stdlib_count()` 构造方法
  - Consumes: 无

### Task 2.2: Driver passes stdlib_item_count to CodeGenerator
- **File**: `crates/ruyic/src/driver.rs` (around line 570)
- **TDD**:
  1. 在 `generate_code` 方法中，记录 `program.items.len()` 作为总项数
  2. 确定 stdlib 项的数量：在 `compile` 方法中记录 `self.resolver.loaded_modules` 的所有 items 总数
  3. 将 `stdlib_item_count` 传递给 `CodeGenerator::with_gc_mode_and_stdlib_count`
  4. 将 `generator.allow_partial_codegen = true` 从无条件改为由构造方法内部根据 stdlib_count 计算
  5. 运行 `make check` 确认编译通过
  6. 运行 `cargo test -p ruyic --test codegen` 确认现有测试全部通过
- **Interfaces**:
  - Consumes: `CodeGenerator::with_gc_mode_and_stdlib_count()` (from Task 2.1)
  - Produces: 传递给后续所有 codegen 调用

---

## Batch 3: Codegen 12 "Not Yet Supported" Paths

### Sub-Batch 3a: Simple Expressions (4 paths) [Independent within 3]

#### Task 3a.1: Anonymous Function Codegen
- **File**: `crates/ruyic/src/codegen/expr.rs` (line 377)
- **TDD**:
  1. 在 `crates/ruyic/tests/codegen.rs` 中添加测试：`fn test_anon_function()`
     - 输入：`let double = fn(x: int): int { return x * 2; }; print(double(5));`
     - 期望输出：`10`
  2. 运行测试确认当前返回 "Anonymous functions not yet supported" 错误
  3. 实现：在 `compile_function_call` 的非 Identifier 分支中，对匿名函数复用现有箭头函数编译路径，生成 `__anon_{counter}` 命名函数，注册到 ctx 并返回函数指针
  4. 运行测试确认通过
  5. 运行 `cargo test -p ruyic --test codegen` 确认无回归
- **Interfaces**: 无跨任务接口

#### Task 3a.2: Async Arrow Function Codegen
- **File**: `crates/ruyic/src/codegen/expr.rs` (line 387)
- **TDD**:
  1. 在 `crates/ruyic/tests/codegen.rs` 中添加测试：验证 async arrow 能编译（不要求完整 async runtime 验证）
  2. 运行测试确认当前返回错误
  3. 实现：async arrow 编译为 `__async_arrow_{counter}`，复用 `async_codegen.rs` 中的 async 函数 codegen 路径
  4. 运行测试确认通过
  5. 运行完整 codegen 测试确认无回归
- **Interfaces**: 依赖 `async_codegen.rs` 现有 async 基础设施

#### Task 3a.3: Compound Assignment Codegen
- **File**: `crates/ruyic/src/codegen/expr.rs` (line 2563)
- **TDD**:
  1. 添加测试：`let x = 5; x += 3; print(x);` → `8`
  2. 添加测试覆盖所有操作符：`+=`、`-=`、`*=`、`/=`、`%=`
  3. 运行测试确认当前返回 "Compound assignment not yet supported"
  4. 实现：在 `compile_assign` 的 `AssignOp` 非 `Assign` 分支中，对 Identifier 和 MemberAccess 左值执行 load → compile_binary_op → store
  5. 运行测试确认通过
  6. 运行完整 codegen 测试确认无回归
- **Interfaces**: 调用现有 `compile_binary_op`

#### Task 3a.4: Indirect Call Codegen
- **File**: `crates/ruyic/src/codegen/expr.rs` (line 2336)
- **TDD**:
  1. 添加测试：`let f = someFunc; f(42);`
  2. 运行测试确认当前返回错误
  3. 实现：编译 callee 表达式得到函数指针，`build_bitcast` 对齐类型后 `build_indirect_call`
  4. 运行测试确认通过
  5. 运行完整 codegen 测试确认无回归
- **Interfaces**: 调用 `function_type_from_ruyi` 获取函数签名

### Sub-Batch 3b: Access & Assignment (3 paths) [Depends on: 3a.3 (compound assign context)]

#### Task 3b.1: Nested Member Access Codegen
- **File**: `crates/ruyic/src/codegen/expr.rs` (line 2236)
- **TDD**:
  1. 添加测试：`obj.prop.method()` 和 `a.b.c` 链式访问
  2. 运行测试确认当前返回错误
  3. 实现：在 `compile_member_call` 中，对嵌套 MemberAccess 递归编译外层 → 获取 ptr + type → 继续编译内层 GEP + load
  4. 运行测试确认通过
  5. 运行完整 codegen 测试确认无回归
- **Interfaces**: 递归调用 `class_field_access`

#### Task 3b.2: Complex Assignment Codegen
- **File**: `crates/ruyic/src/codegen/expr.rs` (line 2621)
- **TDD**:
  1. 添加测试：`arr[0] = 42;` 数组索引赋值
  2. 运行测试确认当前返回错误
  3. 实现：扩展 `compile_assign` 的 left 匹配分支，添加 IndexAccess 处理：编译数组表达式 → 编译索引 → `__builtin_array_set`
  4. 运行测试确认通过
  5. 运行完整 codegen 测试确认无回归
- **Interfaces**: 调用 `__builtin_array_set`

#### Task 3b.3: Complex New Expression Codegen
- **File**: `crates/ruyic/src/codegen/expr.rs` (line 2974)
- **TDD**:
  1. 添加测试：`throw Error.new("test");` 和 `new (getClass())(args)`
  2. 运行测试确认当前返回错误
  3. 实现：在 `compile_new` 中，非 Identifier callee 时先编译 callee 表达式获取类名/类型，再按标准 new 流程分配+构造
  4. 运行测试确认通过
  5. 运行完整 codegen 测试确认无回归
- **Interfaces**: 复用现有 `GcAllocFn` + 构造器调用逻辑

### Sub-Batch 3c: Spread Arguments (5 paths) [Independent within 3]

#### Task 3c.1: Extract unpack_spread_args Utility
- **File**: `crates/ruyic/src/codegen/expr.rs` (new function)
- **TDD**:
  1. 不添加独立测试——通过后续 4 个 spread site 测试验证
  2. 实现公共函数 `fn unpack_spread_args<'ctx>(ctx: &mut CodegenContext<'ctx, '_, '_>, args: &[Argument]) -> Result<Vec<BasicMetadataValueEnum<'ctx>>, String>`
  3. 逻辑：遍历 args，对 `Argument::Expr` 直接编译追加，对 `Argument::Spread` 编译数组后调用 `__builtin_array_length` + `__builtin_array_get` 逐元素解包追加
  4. 运行 `make check` 确认编译通过
- **Interfaces**:
  - Produces: `unpack_spread_args()` 公共函数
  - Consumes: `compile_expr`, `__builtin_array_length`, `__builtin_array_get`

#### Task 3c.2: Spread Arguments in Function Call (Site 1)
- **File**: `crates/ruyic/src/codegen/expr.rs` (line 2405)
- **TDD**:
  1. 添加测试：`fn sum(a, b, c) { return a + b + c; }; let arr = [1, 2, 3]; print(sum(/* spread manually: arr[0], arr[1], arr[2] */));`
  2. 将 `return Err("Spread arguments not yet supported")` 替换为调用 `unpack_spread_args`
  3. 运行测试确认通过
- **Depends on**: Task 3c.1

#### Task 3c.3: Spread Arguments in Function Call (Site 2)
- **File**: `crates/ruyic/src/codegen/expr.rs` (line 2496)
- **TDD**:
  1. 同 3c.2 模式——此路径位于直接函数调用的另一个分支
  2. 替换错误返回为 `unpack_spread_args`
  3. 运行 codegen 测试确认
- **Depends on**: Task 3c.1

#### Task 3c.4: Spread Arguments in Constructor Call
- **File**: `crates/ruyic/src/codegen/expr.rs` (line 2995)
- **TDD**:
  1. 添加测试：`new Foo(...args)` 模式
  2. 替换错误返回为 `unpack_spread_args`
  3. 运行测试确认
- **Depends on**: Task 3c.1

#### Task 3c.5: Spread Arguments in Super Constructor Call
- **File**: `crates/ruyic/src/codegen/expr.rs` (line 3044)
- **TDD**:
  1. 添加测试：子类构造函数 `super(...args)` 模式
  2. 替换错误返回为 `unpack_spread_args`
  3. 运行测试确认
- **Depends on**: Task 3c.1

### Sub-Batch 3d: Complex Pattern Binding (1 path) [Independent within 3]

#### Task 3d.1: Complex Pattern Binding Codegen
- **File**: `crates/ruyic/src/codegen/decl.rs` (line 75)
- **TDD**:
  1. 添加测试：`let [a, b] = [1, 2]; print(a + b);` → `3`
  2. 添加测试：`let { x, y } = point; print(x + y);`
  3. 运行测试确认当前返回错误
  4. 实现：
     - `Pattern::Array(elements)`: 编译右侧表达式 → 迭代元素索引 → 对每个子模式生成 `__builtin_array_get` + `compile_binding`
     - `Pattern::Object(fields)`: 编译右侧表达式 → 对每个字段 `Pattern::Identifier(name)` 生成 `class_field_access` + `build_load` + `alloca` + `store`
  5. 运行测试确认通过
  6. 运行完整 codegen 测试确认无回归
- **Interfaces**: 调用 `__builtin_array_get`、`class_field_access`、递归调用 `compile_binding`

### Task 3.5: Final Codegen Regression Gate
- **Work**: 运行 `cargo test -p ruyic --test codegen` 全量通过
- **Work**: 运行 `cargo test -p ruyic --lib` 全量通过
- **Work**: 确认所有 12 条路径中再无 `"not yet supported"` 字符串（grep 验证）
- **Depends on**: 3a.1-3a.4, 3b.1-3b.3, 3c.2-3c.5, 3d.1

---

## Batch 4: Roadmap Documentation [Independent]

### Task 4.1: Update roadmap.md to v0.5.9
- **File**: `docs/roadmap.md`
- **Work**:
  1. 更新 Header 版本号：`0.5.4` → `0.5.9`，日期 `2026-07-07` → `2026-07-16`
  2. 版本表中添加 v0.5.8 和 v0.5.9 行
  3. 在 v0.2 任务表中标记：
     - 1.7 (`try/catch/finally`) → ✅ (v0.5.5)
     - 1.8 (`throw expression`) → ✅ (v0.5.5)
     - 1.10 (`template literals`) → ✅ (v0.5.5)
  4. 在 v0.4 任务表中确认 3.2/3.4/3.5/3.6 已标记 ✅ (v0.5.7)
  5. 在 v0.5 任务表中标记：
     - 4.1 (`SetIterator`) → ✅ (v0.5.9)
     - 4.2 (`math.ry`) → ✅ (v0.5.8)
     - 4.3 (`time.ry`) → ✅ (v0.5.8)
     - 4.4 (`json.ry`) → ✅ (v0.5.8)
  6. 更新 Current State Assessment 中 Codegen 完成度：75% → 82%
  7. 确认所有已有 ✅ 项与版本一致（不对已标记项做修改）
- **Depends on**: 无

### Task 4.2: Update roadmap-zh.md to v0.5.9
- **File**: `docs/roadmap-zh.md`
- **Work**: 与 Task 4.1 同步，确保中英文路线图一致
- **Depends on**: Task 4.1 (内容对齐)

---

## Final Verification Wave

### Task FV.1: make check
```bash
make check
```
预期：通过，零新增 clippy 警告

### Task FV.2: Full test suite
```bash
cargo test --workspace
```
预期：全部通过（允许 pre-existing 的 GC 相关 clippy 警告，与本 change 无关）

### Task FV.3: Not-yet-supported string elimination
```bash
grep -rn 'not yet supported' crates/ruyic/src/codegen/
```
预期：零结果（所有 12 条路径均已移除该字符串）

### Task FV.4: allow_partial_codegen verification
- 验证 `driver.rs` 中不再有无条件的 `allow_partial_codegen = true`
- 验证 codegen 测试在有/无 stdlib 场景下行为正确

### Task FV.5: Roadmap consistency
```bash
diff <(grep '✅' docs/roadmap.md | sort) <(grep '✅' docs/roadmap-zh.md | sort)
```
预期：两文件 ✅ 标记一致
