# 执行合同：v0.5.7-p1-defects

> **Change**: v0.5.7-p1-defects | **Branch**: `dev/v0.5.7-p1-defects` | **Tag**: `v0.5.7`
> **State**: specifying → bridging (DP-2 已批,等待 DP-3 用户确认本合同)
> **Workflow**: full (强制 SDD)

## Intent Lock

- **变更名称**：v0.5.7-p1-defects
- **要解决的问题**：9 项 P1 缺陷长期阻塞 Ruyi 写出生产级程序，跨 typechecker 4 项（3.2 supertraits 传递链不闭合 / 3.4 narrowing 仅覆盖 `=== null` / 3.5 `Type::Union` match 缺臂无诊断 / 3.6 裸 `Self` 字段一刀切拒绝）、runtime 1 项（2.6 挂起 async task 的 GC roots 是 no-op）、stdlib 4 项（4.5 random / 4.6 fmt / 4.8 test 模块缺失；4.9 collections 缺 ~15 方法）。
- **范围内**：9 项 P1 特性项（3.2 / 3.4 / 3.5 / 3.6 / 2.6 / 4.5 / 4.6 / 4.8 / 4.9）的实现 + 配套 Javadoc / 测试 / integration 示例 / 文档同步。3 项 doc-drift（1.8 Throw / 1.9 Match / 1.10 Template literal）维持 v0.5.6-codegen-doc-drift 已关闭状态。
- **范围外**：P2/P3 缺陷（4.7 regex、4.10 core+string 合并、4.11 buffer、4.12 net、2.7 thread-local GC、1.11 BigInt literal 等）；新语言特性 / 新 GC 算法 / async runtime 重写；`__io_*` / `__process_*` / `__path_*` 历史未声明 FFI 清理；CI/CD 基础设施、criterion benchmark 套件；任何破坏性语言变化。

## Approved Behavior

### 已批准需求摘要（9 项 × 1-2 行）

| 编号 | 特性项 | 行为摘要 |
|------|--------|----------|
| 3.2 | supertraits | `TraitDecl.supertraits` 解析并按声明顺序保留；`validate_impls` 沿 supertrait 链收集全部方法；任意深度（≥3 节点）传递循环在编译期报错。 |
| 3.4 | narrowing | `if (x !== null)` true 分支正向收窄为 `T`；`else` 分支反向收窄回 `T?`；`instanceof T` / `typeof` / match 模式三种新窄化源独立生效；收窄不跨循环泄漏。 |
| 3.5 | exhaustiveness | `Type::Union` + `Expr::Match` 缺臂产出诊断，文本含缺失 Variant 名；通配 `_` 抑制缺臂诊断；诊断维持 warning 级以保持向后兼容。 |
| 3.6 | self-referential | `Box<Self>` / `Option<Self>` / `List<Self>` 等间接自引用类字段可通过编译；裸 `Self` 字段维持 v0.5.5 拒绝行为；递归深度阈值防无限展开。 |
| 2.6 | async GC roots | `ruyi_gc_collect` 前调用 `register_async_roots`，遍历 `Scheduler::suspended_tasks()` 把 future 链加入根集合；任务结束后引用自动解除。 |
| 4.5 | random | `stdlib/random.ry` 导出 `Random` 类 + 5 API（`random_new`/`nextInt`/`nextFloat`/`nextBool`/`nextBytes`）；5 个 `ruyi_random_*` C FFI 绑定；`nextInt` 在 `min===max` 返回 `min`、`min>max` 报错。 |
| 4.6 | fmt | `stdlib/fmt.ry` 导出 `format`/`println`；`{}` 顺序占位 + `{n}` 下标占位 + `{name}` 命名占位；底层走 `__string_replace_all` + `core.ry` 既有 toString。 |
| 4.8 | test | parser 识别 `@test` 属性前缀并写入 `TestFunctionRegistry`；`stdlib/test.ry` 导出 4 个断言函数；`ruyic --test` CLI 调度并按失败数返回退出码。 |
| 4.9 | collections ext | `stdlib/collections.ry` 新增 Array ≥7 + Iterator ≥8 方法；`sum`/`product` 需 `Add`/`Mul` supertrait；**严格 BLOCKED on 3.2 supertraits**（`trait Add` 声明须先通过 cycle 检测）。 |

### 关键场景（每项 1 行）

- **3.2** `trait A extends B / B extends C / C extends A` → 编译错误列出全部 3 节点。
- **3.4** `if (animal instanceof Dog) { animal.bark(); }` → `animal` 在分支内收窄为 `Dog`。
- **3.5** `match (color: Red|Green|Blue) { Red => ..., _ => ... }` → 不产出缺臂诊断。
- **3.6** `class Node { next: Node?; }` → 编译通过；`class Bad { me: Self; }` → 报错含 `Self` 字面量。
- **2.6** 1000 个对象 + 10 个挂起 task 各持引用 → `gc_collect()` 后全部存活。
- **4.5** `r.nextInt(5, 5)` → 始终返回 `5`；`r.nextInt(10, 5)` → 报错或 panic。
- **4.6** `fmt.format("Hello, {user}", {"user": "world"})` → `"Hello, world"`。
- **4.8** `--test` 模式下 4 个 `@test fn` 通过 → 退出码 0；任一断言失败 → 退出码 = 失败数。
- **4.9** `[1,2,3,4].sum()` → `10`（int 实现 `Add`）；`[1,2].product()` → `2`。

### 验收检查（DP-1 12 项，摘自 `proposal.md` Acceptance）

1. 9 项 P1 缺陷全部关闭，每项有专属测试通过（`supertraits_cycle` / `narrowing_reverse` / `exhaustiveness_union` / `self_referential` / `async_gc_roots` / `random_ffi` / `fmt_ffi` / `parser_test_attr` / `collections_arrayops`）。
2. `cargo test --workspace` 全绿，无新增 regression。
3. `cargo clippy --workspace --all-targets -- -D warnings` 零新警告。
4. `stdlib/collections.ry` 新增 Array + Iterator 方法 ≥15 个（DP-1 下限）。
5. `crates/ruyic/src/runtime/random_ffi.rs` 中 `#[no_mangle] pub extern "C"` ≥5 个（`seed`/`int`/`range`/`choice`/`shuffle`）。
6. parser 支持 `@test` 属性在 `fn` 前缀并提供 `TestFunctionRegistry::collect_from_program` 入口。
7. `ruyi_gc_collect` 在挂起 task 持有引用时正确保留 GC 对象（`async_gc_roots.rs`）。
8. `Type::Union` + `Expr::Match` 启用 exhaustiveness 检查，缺臂产出 warning。
9. `docs/spec.md` / `docs/roadmap.md` / `docs/roadmap-zh.md` / `CHANGELOG` 全部同步。
10. `cargo fmt --check` 通过。
11. `dev/v0.5.7-p1-defects` 分支上的 v0.5.7 release commit 完成。
12. 按 AGENTS.md 分支策略以 merge commit 形式合并到 `main` + annotated tag `v0.5.7`。

## Design Constraints

- **架构约束**：
  - 单 change 内嵌 4 个并行 sub-batch（Typechecker / Runtime / Stdlib-fast / Stdlib-heavy），单 release commit。
  - 严格顺序：**Sub-batch 1.1 supertraits 必须在 Sub-batch 4.2 collections 之前合并**（`trait Add` 须先通过 cycle 检测）；3.2 内部不再有子顺序。
  - Supertrait 传递循环检测用 DFS 白/灰/黑三色着色（手写，不引入 petgraph）。
  - Stdlib random 全部走 `ruyi_random_*` C FFI，不在 `.ry` 内部持有原生逻辑；token 缓冲固定 32/64 字节。
- **接口约束**（摘自 `tasks.md` Interfaces 节，7 项跨 batch Consumes/Produces）：
  1. **1.1 → 4.2**：Batch 1 产出 `TraitRegistry::has_cycle(&str) -> bool` + `DiagnosticKind::SupertraitCycle { chain: Vec<String> }`；Batch 4 `trait Add` 编译前消费之。
  2. **1.2 → 4.2**：Batch 1 产出 `narrowing::apply_reverse_narrow(env, name, original_ty, narrowed)`；Batch 4 `partition` / `find_index` 在 else 分支消费之。
  3. **1.3 → 4.2 + stdlib**：Batch 1 产出 `exhaustiveness::check_union(&Type, &[MatchArm]) -> ExhaustivenessReport { is_exhaustive, missing_cases, redundant_arms }`；Batch 4 `assert_eq` 与 `partition` 消费之。
  4. **1.4 → 4.2**：Batch 1 产出 `self_ty::resolve(ann, &ElementContext) -> Option<Type>`；Batch 4 收集方法 Self 感知返回类型。
  5. **2.1 → 4.1**：Batch 2 产出 `async_gc_roots::register_suspended_task(task_id, stack_base)` / `unregister_suspended_task(task_id)`；Batch 4 测试运行器在 `@test fn` 挂起时消费。
  6. **3.2 → 4.2 方法 4.9**：Batch 3 产出 `fmt::format(value, spec) -> string`；Batch 4 `Iterator.filter` / `partition` 错误信息消费之。
  7. **1.1 → 4.2 BLOCKED**：Sub-batch 4.2 全部 5 子任务 `Depends on: Sub-batch 1.1`，由 `tasks.md` + PR description 双写显式声明。
- **依赖约束**：LLVM 14（macOS `brew install llvm@14` + `LLVM_SYS_140_PREFIX`）；Rust 2021 edition、workspace resolver="2"；`inkwell` 的 `llvm14-0` feature；不引入 petgraph / criterion / proptest 等新外部 crate；clippy zero-warning（AGENTS.md 零警告原则）；保留所有既有 `/** ... */` Javadoc。
- **数据约束**：`TraitDecl.supertraits` 字段扩展（向后兼容、default empty）；`NarrowEnv` 状态枚举新增变体；`TypeChecker` 暴露 `register_test_function()`；`Runtime::gc_collect()` 内部签名变化但外部 API 不变；`Compiler` CLI 增量加 `--list-tests` / `--run-tests` 标志。

## Task Batches

### Batch 1: Typechecker（4 features × 5 TDD 步骤）

- **目标**：完成 3.2 / 3.4 / 3.5 / 3.6 四项类型检查器 P1，产出可运行的 DFS 循环检测、反向收窄、缺臂诊断、间接 Self 解析四个新模块。
- **输入**：当前 `crates/ruyic/src/typechecker/` 既有代码；9 个 spec 文件中 4 个 typechecker spec（`supertraits` / `narrowing` / `exhaustiveness` / `self-referential`）。
- **输出**：4 个新模块文件 `supertraits.rs` / `narrowing.rs` / `exhaustiveness.rs` / `self_ty.rs`；4 处既有文件修改（`traits.rs` / `inference.rs` / `patterns.rs` / `types.rs`）；4 个新测试文件（`crates/ruyic/tests/` 下 4 个 integration）；4 个 conventional commit（`feat(typechecker): ...`）。
- **完成标准**：全部 4 个 sub-batch 的 TDD 5 步（写失败测试 → 运行确认失败 → 实现最小化代码 → 运行确认通过 → 提交）全绿，`cargo test --workspace` 无回归，`cargo clippy --workspace -- -D warnings` 零新警告。

### Batch 2: Runtime（1 feature × 5 TDD 步骤）

- **目标**：完成 2.6 async GC roots，把 `register_async_roots` 从 no-op 改为真实遍历挂起 task 并将 future 链加入 GC 根集合。
- **输入**：当前 `crates/ruyi_runtime/src/gc_exports.rs` + `async_runtime.rs`；spec `async-gc-roots/spec.md`。
- **输出**：1 个新文件 `crates/ruyic/src/runtime/async_gc_roots.rs`（含 `AsyncGcRoots` 结构 + 2 个 `extern "C"` 入口）；2 处既有文件修改（`gc_exports.rs` 在 `ruyi_gc_collect` 内调用 `GLOBAL_ROOTS.snapshot()`；`async_runtime.rs` 在 `TaskSuspend` 处调用 `ruyi_async_register_root`）；1 个测试文件 `crates/ruyi_runtime/tests/async_gc_roots.rs`；1 个 conventional commit（`feat(runtime): ...`）。
- **完成标准**：TDD 5 步全绿；payload 在 `gc_collect()` 后仍存活；`cargo test -p ruyi_runtime --test async_gc_roots -- --nocapture` PASS；不引入新 lock contention（Mutex 持有时间 O(1)）。

### Batch 3: Stdlib-fast（2 features × 5 TDD 步骤，可与 Batch 1/2 并行）

- **目标**：完成 4.5 random 与 4.6 fmt 两个独立 stdlib 模块，提供 5 + 3 = 8 个 C FFI 符号与对应 `.ry` 包装。
- **输入**：spec `random-stdlib/spec.md` + `fmt-stdlib/spec.md`；当前 `crates/ruyic/src/runtime/builtins.rs` FFI 注册模式。
- **输出**：2 个新文件 `stdlib/random.ry` + `stdlib/fmt.ry`；2 个新 FFI 文件 `random_ffi.rs` + `fmt_ffi.rs`；2 个测试文件 `crates/ruyi_runtime/tests/random_ffi.rs` + `fmt_ffi.rs`；1 处 `lib.rs` re-export；2 个 conventional commit。
- **完成标准**：TDD 5 步全绿；`nm <binary> | grep ruyi_random_` 命中 5 个符号；`nm <binary> | grep ruyi_fmt_` 命中 3 个符号（`format_int` / `format_float` / `pad_right`）；link-time 零 undefined。

### Batch 4: Stdlib-heavy（2 features × 7 TDD 步骤，BLOCKED on Batch 1.1）

- **目标**：完成 4.8 test 框架与 4.9 collections 扩展（Array 15 + Iterator 5 = 20 新方法）。
- **输入**：Batch 1.1 完成的 `TraitRegistry::has_cycle`；spec `test-stdlib/spec.md` + `collections-ext/spec.md`；当前 `crates/ruyic/src/parser/ast.rs` `Declaration::Function` 结构；`stdlib/collections.ry` 既有 `ArrayOps` / `Iterator` trait。
- **输出**：1 个新文件 `crates/ruyic/src/runtime/test_registry.rs`；3 处既有文件修改（`parser/ast.rs` 给 `Function` 加 `annotations: Vec<String>` 字段；`parser/parser.rs` 调 `parse_annotations()`；`typechecker/checker.rs` 在 check 末调用 `collect_from_program`）；2 个新 stdlib 文件 `stdlib/test.ry`；`stdlib/collections.ry` 末尾追加 20 个方法签名 + 5 个迭代器类（`FilteredIterator` / `TakeWhileIterator` / `SkipWhileIterator` / `EnumeratedIterator` / `ChainedIterator`）；1 个新测试文件 `crates/ruyic/tests/parser_test_attr.rs` + `collections_arrayops.rs`；3 个 conventional commit。
- **完成标准**：TDD 5 步全绿；`cargo test --workspace` 全绿；Array 15 方法 + Iterator 5 方法均编译通过；`trait Add` 在 `collections.ry` 中无 cycle 报错；与 Batch 1.1 合并顺序由 `git log --grep "3.2 supertraits"` 早于 4.9 commit 验证。

## Test Obligations

- **必须先从失败测试开始的行为**：每个 sub-batch 的 TDD 5 步是强制 gate（tasks.md 已对 50 个原子任务逐一定义）。Step 1.2 / 2.2 / 3.2 / 4.2 / 4.9 必须显式运行失败测试并把 stderr 输出贴入 PR description；任何"已经实现再写测试"或"跳过失败确认"的行为触发 review 拒收。失败信息中需明确写出"当前 X 模块尚未存在 / 当前 Y 行为不存在"的根因（见 `tasks.md` 每个 Step 1.2/2.2/3.2 的 `Expected: FAIL` 注释）。
- **必需的边界情况**（来自 specs/ 的 WHEN/THEN，6 项核心边界）：
  1. **supertraits 三节点循环**（`specs/supertraits/spec.md` Scenario "循环发生在超 trait 链的更深层"）：`A extends B / B extends C / C extends A` 必须列出全部 3 节点。
  2. **narrowing 跨循环不泄漏**（`specs/narrowing/spec.md` Scenario "循环外收窄不会泄漏"）：`if` 收窄后 `for (...) { use(x); }` 内 `x` 仍为 `T?`。
  3. **exhaustiveness warning 不阻塞产物**（`specs/exhaustiveness/spec.md` Scenario "warning 不阻塞产物生成"）：缺臂 match 仍生成可执行二进制，退出码 0。
  4. **async GC 多层 future 链穿透**（`specs/async-gc-roots/spec.md` Scenario "多层 future 链引用穿透"）：深度 ≥3 的 `await` 链中引用全部被 mark。
  5. **random min === max**（`specs/random-stdlib/spec.md` Scenario "min 与 max 相同"）：`r.nextInt(5, 5)` 调用 100 次全部返回 `5`。
  6. **collections supertrait 拒绝**（`specs/collections-ext/spec.md` Scenario "缺失 supertrait 拒绝"）：类型参数 `T` 调用 `.sum()` 缺 `Add` 时报错，诊断定位在调用点行号。
- **回归敏感区域**：
  - **typechecker inference**：3.4 narrowing 影响范围最大（涉及 `inference.rs::narrow_for_condition` 与 merge point join）；2400+ 既有测试需保持全绿。
  - **GC root marking**：2.6 把 `register_async_roots` 接到 `collect_full` mark 阶段前，需确认 stop-the-world 暂停时间可接受，且 `ruyi_runtime` 测试中既有 GC 用例不回归。
  - **supertrait cycle**：3.2 的 DFS 检测在 `trait A {}`（无 supertrait）情况必须不误报；既有 `trait Comparable {}` 等无依赖 trait 必须通过。
  - **parser `@test` 语法**：4.8 改 `Declaration::Function` 加 `annotations` 字段，AST 结构变化需 golden fixture 快照回归。
  - **clippy 警告**：所有 50 个新增函数 / 模块必须 `-D warnings` 通过，不得引入 `dead_code` / `unused_imports` / `needless_return` 等。

## Execution Mode

- **模式**：`SDD`（Spec-Driven Development）
- **选择理由**：
  1. spec-superflow workflow=`full` 强制 SDD 路径（参见 `customize-opencode` 状态机）。
  2. 单 change 含 4 个并行 sub-batch + 1 个严格顺序约束（3.2 → 4.9），Inline / Batch Inline 无法表达 sub-batch 间依赖与跨 batch 接口契约。
  3. 9 项 P1 跨 typechecker / runtime / stdlib 三大 crate，4-5 周周期，SDD 提供的 50 个原子 TDD 任务 + 30 条 spec Requirement + 12 项 DP-1 验收是 batch 边界与质量门的唯一可信依据。
  4. 7 项跨 batch Consumes/Produces 接口（见 Design Constraints）需要每个 sub-batch 开始前检查依赖项已落地，Inline 模式无此 gate。

## Verification Dimensions

| 维度 | 状态 | 发现 |
|------|------|------|
| Completeness | Pending | 30/30 spec Requirement 全部映射到 TDD 步骤与 batch；7/7 跨 batch 接口已声明 Consumes/Produces；12/12 DP-1 验收项已对应到验证手段。 |
| Correctness | Pending | 边界 6 项已从 specs/ WHEN/THEN 抽取；回归敏感区域 5 项已列出；TDD 5 步强制 gate 在 `tasks.md` 50 任务中逐一定义。 |
| Coherence | Pending | Batch 1 / 2 / 3 可并行启动，Batch 4 由 Batch 1.1 显式 BLOCKED；specs/30 条 Requirement 与 proposal.md 9 项 P1 一一对应；design.md 5 项决策与 tasks.md Interfaces 节一致。 |

**总体结论**：Pending（等待 DP-3 用户批准后转入 bridging → implementing）。

## Review Gates

- **强制审查点**：
  1. **Per-batch review**：每个 batch 完成后由 typechecker owner / runtime owner / stdlib owner 分别审 batch 内的所有 commit，按 ssf-code-reviewer skill 执行，输出 review verdict。
  2. **Inter-batch dependency check**：在 Batch 4.2 第一个 commit 合并前，`git log --grep "3.2 supertraits"` 必须命中且在 Batch 4.2 commit 之前；否则 PR review 拒收。
  3. **Interface conformance check**：Batch 4 PR review 时对照 `tasks.md` Interfaces 节检查 7 个跨 batch 接口的 Rust 签名是否完全一致（`TraitRegistry::has_cycle` / `narrowing::apply_reverse_narrow` / `exhaustiveness::check_union` / `self_ty::resolve` / `async_gc_roots::register_suspended_task` + `unregister_suspended_task` / `fmt::format`）。
  4. **Javadoc 保留检查**：PR diff 中若删除任何 `/** ... */` 行则 review 拒收（AGENTS.md Javadoc 保留原则）。
  5. **Release-gate closeout**：执行 `proposal.md` 12 项验收清单 + `make check` + `make build-release` + `make test` + `make lint` + `make fmt-check` + `make run-example EXAMPLE=random` + `make run-example EXAMPLE=collections`，全部通过方可打 tag `v0.5.7`。
- **阻塞类别**：
  - clippy regression（任何新增 warning / 既有 warning 复活）
  - test failure（任一 TDD Step 1.4 / 2.4 / 3.4 / 4.4 / 4.6 / 4.11 / 4.14 在第三次尝试后仍失败）
  - contract drift（`tasks.md` 文件路径、接口签名、commit message 与本合同的 Design Constraints / Task Batches 节出现不一致）
  - ordering violation（Batch 4.2 commit 早于 Batch 1.1 在 `git log` 中出现）
  - Javadoc 删除（违反 AGENTS.md 零警告原则的延伸约束）

## Escalation Rules

- **何时回退到 `specifying`**：
  - 任一 9 项 P1 的 spec Requirement 在实现阶段被识别为定义模糊或缺失场景（如 3.4 narrowing 在 `dyn` 联合上的行为 spec 未覆盖）。
  - 新增需求：实现过程中发现必须额外关闭某 P2 缺陷才能完成 P1（如 4.9 collections 强制需要 core.ry 的 `string_concat` 重构）。
  - 设计缺陷：DFS 着色检测 supertrait cycle 在工程实现中遇到无法在 50 行内表达的边界（如宏生成的 trait / 条件 trait）。
  - 新增跨 batch 依赖：发现 Batch 4 还需依赖 Batch 2（当前未声明），需修改 design.md 决策并重新过 DP-2。
- **何时回退到 `bridging`**：
  - `contract_hash` 与 proposal.md / design.md / tasks.md / specs/ 的当前 hash 不匹配（contract 漂移）。
  - 用户在 DP-3 阶段对本合同任一节提出修改且影响 Design Constraints 或 Task Batches 的边界。
  - Batch 1 / 2 / 3 之一在并行启动后整体失败（非单 sub-batch 失败），需重新对齐跨 batch 接口。
- **何时不得继续实现**：
  - 任一 TDD Step 在第三次尝试后仍失败（按 review-work skill 触发 systematic-debugging）。
  - `cargo test --workspace` 出现新增 regression 而非 pre-existing。
  - Batch 4.2 启动时 `git log --grep "3.2 supertraits"` 无 commit。
  - 任何 clippy warning 出现且无法通过 1 行本地修复消除。
  - `make build-release` 在 LLVM 14 环境下失败（必须确认 `LLVM_SYS_140_PREFIX` 设置正确后再继续）。