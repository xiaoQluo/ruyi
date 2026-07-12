# 变更提案：v0.5.7-p1-defects

## 背景（Why）

Ruyi v0.5.5 已修复 7 项 P0 并发布；v0.5.6-codegen-doc-drift 关闭了 1.8 Throw、1.9 Match、1.10 Template literal 三项 P1 doc-drift。剩余 9 项 P1 缺陷跨越类型检查器、运行时、标准库三大模块，长期阻塞 Ruyi 写出生产级程序：

- **类型检查器（4 项）** — 3.2 supertraits：`TraitDecl.supertraits` 占位空 Vec、supertrait 路径无验证；3.4 narrowing：仅 `=== null` 一条窄化源，`instanceof`/`typeof`/match pattern 全部缺失；3.5 exhaustiveness：`Type::Union` match 缺臂不报错；3.6 self-referential：类字段引用 `Self` 一律拒绝，连 `Box<Self>`/`Option<Self>` 也不允许。
- **运行时（1 项）** — 2.6 async GC roots：`register_async_roots()` 空实现，挂起协程持有的堆对象在 `ruyi_gc_collect` 时被误回收，存在内存安全风险。
- **标准库（4 项）** — 4.5 random、4.6 fmt、4.8 test 三个模块完全缺失；4.9 collections 缺 ~15 个常用方法（`Array.sort`/`.contains`/`.indexOf`/`.first`/`.last`/`.slice`/`.concat` + `Iterator.takeWhile`/`.skipWhile`/`.chain`/`.enumerate`/`.zip`/`.sum`/`.product`/`.any`/`.all`）。

9 项同时影响 trait/泛型可靠性、async GC 正确性、stdlib 基本功。`docs/roadmap.md` 仍将它们标为 P1，本变更集中收尾并同步 `docs/spec.md`、`docs/roadmap.md`、`docs/roadmap-zh.md` 与 `CHANGELOG`，保证文档与实现一致。当前为合适窗口：v0.5.5 已稳、doc-drift 已清，剩余 9 项定义清晰且无需新语言特性。

## 变更内容（What Changes）

### 子批次 A — 类型检查器（4 项并行）

#### A.1 3.2 supertraits 检查
- `TraitDecl.supertraits` 由占位空 Vec 改为实际解析 `trait Foo: Bar + Baz`
- 实现 `check_supertrait_bounds()`，传递闭包由 `resolve_supertrait_chain()` 返回
- 启用 `tests/typechecker/trait_supertrait.rs` 既有 `#[ignore]` 用例

#### A.2 3.4 narrowing 扩展
- 在 `typechecker::narrow` 新增 `instanceof`、`typeof`、`match pattern` 三种窄化源
- `NarrowEnv` 扩展为三态：`Narrowed(Type)` / `Widened(Type)` / `Unknown`
- 新增 5 个 narrow 用例覆盖 `instanceof T` 与 match pattern binding

#### A.3 3.5 exhaustiveness 检查
- 在 `typechecker::match_check` 对 `Type::Union` 启用全模式覆盖判定
- 缺失分支产出 warning 级诊断（暂不升级为 error，向后兼容）
- 新增 `Type::Union::missing_arms()` 返回缺失 Variant 列表

#### A.4 3.6 self-referential 类
- `typechecker::class::infer_field_types` 允许 `Box<Self>`/`Option<Self>`/`List<Self>` 等间接自引用
- 裸 `Self` 直接字段仍禁止（保持 v0.5.5 行为）
- 新增 `class_self_ref.ry` integration 用例覆盖 `Box<Self>` 与 `Option<Self>`

### 子批次 B — 运行时（1 项）

#### B.1 2.6 async GC roots
- `ruyi_runtime/src/gc/roots.rs` 实现 `register_async_roots()`：遍历 `Scheduler::suspended_tasks()`，对每个 task 的 future 链调用 `GcVisitor::visit_root()`
- `ruyi_gc_collect` 触发 minor/major GC 时先调用 `register_async_roots` 再扫全局根
- 新增 `ruyi_runtime/tests/async_gc_roots.rs`：分配 1000 个对象、`spawn` 10 个挂起任务各持一引用、强制 GC、断言全部可达

### 子批次 C — 标准库 fast（random + fmt）

#### C.1 4.5 random 模块
- 新建 `stdlib/random.ry`：`Random` 类 + `random_new(seed?)` / `nextInt(min,max)` / `nextFloat()` / `nextBool()` / `nextBytes(n)` / `seed(n)` 共 6 个 API
- `crates/ruyi_runtime/src/random.rs` 实现 5 个 FFI：`ruyi_random_new` / `_next_int` / `_next_float` / `_next_bool` / `_next_bytes`
- 默认熵源 `std::collections::hash_map::RandomState`；seed 模式 xorshift64
- 单元测试覆盖 min==max、bytes.length=0、seed 复用等边界

#### C.2 4.6 fmt 模块
- 新建 `stdlib/fmt.ry`：`fmt.format(template, ...args)` + `fmt.println(template, ...args)`
- 支持 `{}` 占位符（顺序）、`{0}`/`{1}` 命名占位符
- 复用 `core.ry` 的 `string_concat` 与 `Int.toString`/`Float.toString`
- 单元测试覆盖基本模板、混合类型、缺失参数报错

### 子批次 D — 标准库 heavy（test + collections 扩展）

#### D.1 4.8 test 模块
- 新建 `stdlib/test.ry`：`assert` / `assertEq` / `assertNotEq` / `assertThrows` / `assertNotNull` / `TestRegistry`
- parser 识别 `@test` 属性，把后续 `fn` 加入 `TestFunctionRegistry`
- `crates/ruyic/src/driver.rs` 增加 `--list-tests` 与 `--run-tests` 编译标志
- 集成测试通过 `--run-tests` 调度 4 个用例

#### D.2 4.9 collections 扩展
- `stdlib/collections.ry` 增加 7 个 Array 方法：`sort(comparator?)` / `contains(elem)` / `indexOf(elem)` / `first()` / `last()` / `slice(start,end?)` / `concat(other)`
- 增加 8 个 Iterator 方法：`takeWhile(pred)` / `skipWhile(pred)` / `chain(other)` / `enumerate()` / `zip(other)` / `sum()` / `product()` / `any(pred)` / `all(pred)`（Array+Iterator 合计 15 个，达 DP-1 下限）
- 单元测试覆盖每个新方法 ≥ 3 个用例

## 能力（Capabilities）

### 新增能力

- `typechecker-supertraits`：trait 继承解析与传递闭包校验
- `typechecker-narrowing-extended`：`instanceof`/`typeof`/match pattern 三类窄化源
- `typechecker-exhaustiveness`：`Type::Union` match 缺臂诊断（warning 级）
- `typechecker-self-ref-classes`：`Box<Self>`/`Option<Self>` 等间接自引用类字段
- `runtime-async-gc-roots`：挂起 task 的 GC 根注册
- `stdlib-random`：`Random` 类与 5 个 runtime FFI
- `stdlib-fmt`：`fmt.format`/`fmt.println` 模板格式化
- `stdlib-test-framework`：`@test` 属性、`TestFunctionRegistry`、`--run-tests` CLI
- `stdlib-collections-extended`：Array + Iterator 共 15 个新方法

### 修改能力

- `typechecker-traits-decl`：`TraitDecl.supertraits` 由占位改实际解析
- `typechecker-narrow`：`NarrowEnv` 状态扩展为三态
- `typechecker-match`：match 检查启用 exhaustiveness 路径
- `typechecker-class-fields`：`infer_field_types` 允许间接 `Self` 引用
- `runtime-gc-collect`：`ruyi_gc_collect` 调用 `register_async_roots` 后再扫描
- `compiler-driver-cli`：新增 `--list-tests`/`--run-tests` 标志
- `compiler-parser-attributes`：识别 `@test` 属性并写入 `TestFunctionRegistry`
- `stdlib-collections`：Array 与 Iterator 方法集合扩充

## 范围（Scope）

### 范围内（In Scope）

- 类型检查器 4 项：3.2 supertraits + 3.4 narrowing + 3.5 exhaustiveness + 3.6 self-referential
- 运行时 1 项：2.6 async GC roots
- 标准库 4 项：4.5 random + 4.6 fmt + 4.8 test + 4.9 collections 扩展（15 个新方法）
- 1.8 Throw / 1.9 Match / 1.10 Template literal 三项 doc-drift 维持已关闭（v0.5.6-codegen-doc-drift）
- 新增 runtime FFI：`ruyi_random_*` 5 个
- 新增 stdlib 文件：`stdlib/random.ry` / `stdlib/fmt.ry` / `stdlib/test.ry`
- `crates/ruyic/src/parser/` 增加 `@test` 属性解析与 `TestFunctionRegistry`
- `docs/spec.md` / `docs/roadmap.md` / `docs/roadmap-zh.md` / `CHANGELOG` 同步更新
- 新增 integration 用例：`class_self_ref.ry` / `random_demo.ry` / `fmt_demo.ry` / `test_demo.ry` / `collections_demo.ry`

### 范围外（Out of Scope）

- P2/P3 缺陷（1.11 BigInt 字面量、2.7 多线程 GC、4.7 regex、4.10 core+string 合并、4.11 buffer、4.12 net 等）—— 后续 change
- 新语言特性：不引入新关键字、新类型、新运算符
- 新 GC 算法设计、不重写 async runtime 架构
- stdlib FFI 清理：`__io_*` / `__process_*` / `__path_*` 等历史未声明符号不在本次范围
- `crates/ruyic/src/codegen/match.rs` 的 integration 测试（DP-1 标 deferred，本变更不展开）
- CI/CD 基础设施（GitHub Actions）、benchmark / property testing 套件
- 任何破坏性语言变化

## 影响（Impact）

### 影响的代码区域

| 区域 | 主要变更 |
|------|---------|
| `crates/ruyic/src/typechecker/` | `traits.rs`（supertraits）、`narrow.rs`（三态）、`match_check.rs`（exhaustiveness）、`class.rs`（self-ref） |
| `crates/ruyic/src/parser/` | 新增 `@test` 属性识别与 `TestFunctionRegistry` 注册 |
| `crates/ruyic/src/driver.rs` | CLI 标志 `--list-tests`/`--run-tests` 与测试调度入口 |
| `crates/ruyic/src/runtime/` | random FFI 绑定声明 |
| `crates/ruyi_runtime/src/` | `gc/roots.rs` 实现 `register_async_roots`、`runtime/random.rs` 实现 5 个 FFI |
| `stdlib/` | 新增 `random.ry`/`fmt.ry`/`test.ry`；`collections.ry` 扩展 15 个方法 |
| `examples/` | 新增 `random_demo.ry`/`fmt_demo.ry`/`test_demo.ry`/`collections_demo.ry` |
| `docs/` | `spec.md` 类型检查与 stdlib 章节、`roadmap.md`/`roadmap-zh.md` P1 状态、`CHANGELOG` |
| `crates/ruyic/tests/` | 新增 `typechecker_self_ref.rs` 等；启用既有 `#[ignore]` |

### 影响的 API 或接口

- `TraitDecl` 字段结构扩展（向后兼容，新字段 default-empty）
- `NarrowEnv` 状态枚举新增变体
- `TypeChecker` 暴露 `register_test_function()` 与 `--run-tests` 入口
- `Runtime::gc_collect()` 内部签名变化（外部 API 不变）
- 新增 stdlib 模块导出 `Random`/`fmt`/`test` 三组 API
- `Compiler` CLI 增加 `--list-tests`/`--run-tests` 两个标志

### 依赖或涉及的外部系统

- **LLVM 14 / inkwell** — 解析器改动触发 codegen 端 `TestFunctionRegistry` 收集，IR 生成路径不变
- **cargo workspace** — 单一 workspace，`ruyi_runtime` 与 `ruyic` 升级版本号至 v0.5.7
- **clippy + rustfmt** — 零新警告（AGENTS.md 强约束），rustfmt 4-space tabs / max_width=100 不变
- **examples 套件** — `examples/run_examples.sh` 扩展为 33 + 4 = 37 个用例

## 验收标准（Acceptance）

按 DP-1 达成的 12 项判定：

1. 9 项 P1 缺陷全部关闭（4 类型检查器 + 1 运行时 + 4 标准库），每项有专属测试通过
2. `cargo test --workspace` 全绿，无新增 regression；既有 `#[ignore]` 用例按计划启用
3. `cargo clippy --workspace` 零新警告
4. `stdlib/collections.ry` 新增方法 ≥ 15 个（Array 7 + Iterator 8）
5. 新增 runtime FFI ≥ 5 个（`ruyi_random_*`）
6. parser 支持 `@test` 属性在 `fn` 声明前，并提供 `TestFunctionRegistry` 收集入口
7. `ruyi_gc_collect` 在挂起 async task 持有引用时正确保留 GC 对象（`async_gc_roots.rs` 通过）
8. `Type::Union` + `Expr::Match` 启用 exhaustiveness 检查；缺臂产出 warning
9. `docs/spec.md` / `docs/roadmap.md` / `docs/roadmap-zh.md` / `CHANGELOG` 全部更新
10. `cargo fmt --check` 通过
11. `dev/v0.5.7-p1-defects` 分支上的 v0.5.7 发布 commit 完成
12. 按 AGENTS.md 分支策略以 merge commit 形式合并到 `main`

### 执行节奏

- 单一 change，4 个并行 sub-batch（Typechecker / Runtime / Stdlib-fast / Stdlib-heavy）
- 顺序约束：3.2 supertraits 先于 4.5–4.9（stdlib 依赖 trait 约束实际工作）；4.9 collections 内部 Array 方法先于 Iterator 方法
- 预计工作量：4–5 周 SDD-mode 执行（与 DP-1 一致）