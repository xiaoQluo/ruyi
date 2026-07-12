# 技术设计 — v0.5.7-p1-defects

**Change:** v0.5.7-p1-defects | **Branch:** `dev/v0.5.7-p1-defects` | **Tag:** `v0.5.7`

## 上下文

- **当前状态**：v0.5.5 已合并；`docs/roadmap.md` 仍把 12 项 P1 缺陷列在 P1 队列：
  - Typechecker：3.2 / 3.4 / 3.5 / 3.6
  - Runtime：2.6 async GC roots
  - Stdlib：4.5 random / 4.6 fmt / 4.8 test / 4.9 collections (~15 方法)
  - 关键文件：`typechecker/{traits,inference,patterns,generics}.rs`、`runtime/{gc,async_runtime,builtins}.rs`、`stdlib/collections.ry`、`parser/` (新增 `@test`)。
- **约束条件**：LLVM 14；Rust 2021、resolver="2"；clippy zero-warning；保留 Javadoc `/** ... */`；3.2 先于 4.9；单 release cycle + no breaking changes；`fmt-check` / `clippy --workspace` / `test --workspace` 全绿。
- **利益相关者**：编译器开发者 (改 typechecker/parser + 加 FFI)、stdlib 用户 (立即获 random/fmt/test + 扩展 collections)、语言用户 (受益 narrowing/exhaustiveness)、发布管线 (单 merge commit 到 main)。

## 目标

1. **3.2** — `traits.rs::validate_impls` 加 DFS coloring 检测 supertrait 传递循环，编译期报错而非 panic。
2. **3.4** — `inference.rs` 引入 per-flow narrowing var（merge 处 join），保留 `if (x !== null) { x.foo() }` 推断形态。
3. **3.5** — `patterns.rs::check_match_exhaustiveness` 验证 match 覆盖全部构造子，未覆盖分支仅 warning。
4. **3.6** — `types.rs` 支持 `self` 出现在字段类型 (`class Node { next: Node?; }`)，含 indirection 检测。
5. **2.6** — `async_runtime.rs::register_async_roots` 不再 no-op，挂起任务栈帧加入 GC root set。
6. **4.5/4.6/4.8** — `random.ry` (5 个 `ruyi_random_*` C FFI)、`fmt.ry` (`format`，占位符 `{}`/`{0}`/`{name}`)、`test.ry` (assert × 4 + `@test` 属性 + `TestFunctionRegistry`)。
7. **4.9** — `collections.ry` 新增 ≥15 方法 (Array.sort/contains/indexOf/first/last/slice/concat; Iterator.takeWhile/skipWhile/chain/enumerate/zip/sum/product/any/all)，依赖 3.2。
8. **Release** — `v0.5.7` 单一 release commit on main；零新增 clippy warning；`docs/roadmap.md` + `docs/spec.md` 同步。

## 非目标

- P2/P3 缺陷修复 (4.10 core+string 合并、4.7 regex、4.11 buffer、4.12 net、2.7 thread-local GC、1.11 BigInt literal)。
- 新 GC 算法设计 (mark-sweep/generational/引用计数之外)。
- 异步运行时架构重写 (work-stealing scheduler 整体替换)。
- 性能基准 / profiling / criterion 新增 benchmark。
- 新 stdlib 模块 (仅 4.5/4.6/4.8)。
- CI/CD 基础设施 (无 GitHub Actions、proptest、cargo-fuzz 集成)。
- 语言级破坏性变更 (语法、类型系统语义变化)。
- stdlib 现有未声明 FFI 清理 (`__io_*` / `__process_*` / `__path_*`，留待下游 change)。

## 决策

### 决策 1: 单一 change 内嵌 4 个 sub-batch (Typechecker / Runtime / Stdlib-fast / Stdlib-heavy)

- **选择**: `v0.5.7-p1-defects` 这一个 change 内拆 4 个并行 sub-batch，仍是单次 release。
- **理由**: 12 个 P1 跨 3 个 crate 但不需独立发布窗口；单 merge commit + 单 tag 沟通开销最低 (与 v0.5.5-residual-fixes 同节奏)；spec-superflow 单 change 比 3-4 change 拆分更易 trace。
- **考虑的替代方案**: 拆 3 个独立 change (Typechecker/Runtime/Stdlib) — 拒绝 (3 cycle + 3 merge，沟通密度稀释)。

### 决策 2: Supertrait transitive cycle detection 用 DFS coloring (而非 post-hoc full scan)

- **选择**: `traits.rs::validate_impls` 对 `TraitDef::supertraits` 字段做白/灰/黑 DFS coloring，发现 back edge 即报错。
- **理由**: incremental DFS 新 `impl` 加入时立即报告，错误回到用户源位置；post-hoc 全图扫描行号丢失到 file:end；DFS 配色对应 `impl_table.rs` 已有 adjacency。
- **考虑的替代方案**: post-hoc 拓扑排序检测 — 拒绝 (定位差、二次扫描)。petgraph 周期检测 — 拒绝 (违反零新增依赖；DFS 30 行手写足够)。

### 决策 3: 逐 flow 独立 narrowing type variable，flow 合并点处做 join

- **选择**: `inference.rs` 为每个 CFG 路径维护独立 `narrow_var`，merge point (`if` 出口/`match` end/loop head) 取 join；join 后 narrow 类型 widening 到上界。
- **理由**: 用户期望 `if (x !== null) { x.foo() }` 分支内窄化为 `T`，到分支外自然 widen 回 `T?`；post-merge widening 对用户透明。
- **考虑的替代方案**: branch widening merge 时立刻恢复声明类型 — 拒绝 (丢失信息，分支出 `match` 后无法保持 narrowed enum)。

### 决策 4: Stdlib random 通过 `ruyi_random_*` 5 个符号的 C FFI 注册

- **选择**: `ruyi_runtime/src/builtins.rs` 注册 5 个 C ABI 符号 (`ruyi_random_*`)，对齐现有 `ruyi_print/format/concat` 模式；`stdlib/random.ry` 只声明 `extern fn` 包装。
- **理由**: 保持 stdlib 与 runtime 职责隔离；5 符号覆盖 `nextInt/nextFloat/nextBool/nextBytes/seed`；`builtins.rs` 已为随机家族预留位置；内部 xoshiro256** 保确定性测试。
- **考虑的替代方案**: 纯 Rust 放 `random.ry` 内部 (零 FFI) — 拒绝 (违反 stdlib 不持 native logic 的隔离原则)。

### 决策 5: 3.2 → 4.9 严格顺序，通过 tasks.md 任务依赖 + PR description 显式声明

- **选择**: tasks.md 中 `4.9 collections extension` task 显式标记 `Depends on: 3.2 supertraits shipped`，PR description + commit message 重复提示。
- **理由**: 4.9 的 `Array.sort(fn(T,T): int)` 回调 trait 需要 supertrait (Comparator is supertrait of Ord)；3.2 未合先合 4.9 会瞬时 trait cycle；显式依赖比 PR review 口头沟通可靠。
- **考虑的替代方案**: 4.9 先做、3.2 后做 — 拒绝 (需 stub supertrait，回头再触 4.9)。并发做 — 拒绝 (merge conflict 高)。

## 风险与权衡

- **Typechecker 回归 (narrowing 影响 2400+ 测试)** → `cargo test --workspace` 全量回归 + truth-table 快照 (≥ 200 路径)。
- **Runtime GC root 泄漏 (async 对象悬挂)** → spawn + GC collect 正反双向断言 reachable。
- **3.2 → 4.9 顺序被绕过** → tasks.md `Depends on: 3.2 shipped` 显式；CI `git log --grep "3.2 supertraits"` 早于 4.9 commit。
- **Parser `@test` 属性触碰 grammar** → golden fixture 快照；新 token 仅在 `@` 位置。
- **Stdlib FFI 符号 mismatch (link-time undefined)** → `builtins.rs` vs `extern fn` 双重对比；`nm | grep ruyi_random_` 确认 5 符号。
- **clippy 新警告 (zero-warning)** → `cargo clippy --workspace --all-targets -- -D warnings` + CI diff 比对 main。
- **Javadoc 删除 (误删)** → diff 含 `/** ... */` 删除则 PR review 拒收。

## 迁移计划

- **上线步骤** (合并前必跑，全部全绿)：
  1. `cargo fmt-check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` — 三件套全绿
  2. `cargo build --release` + 冒烟：新增 stdlib 测试 fixture + 58 集成测试无 regression
  3. 更新 `docs/roadmap.md` (12 P1 ✅)、`docs/spec.md` (`@test` + random/fmt/test 模块)
  4. `git tag -a v0.5.7 -m "Release v0.5.7"` (dev merge commit)；推 `git push origin dev/v0.5.7-p1-defects v0.5.7`；PR dev → main，CI 绿后 merge commit (per AGENTS.md，不 squash 不 rebase)

- **回滚步骤**：
  - **本地未推**：`git reset --hard HEAD~N`
  - **PR 未合**：close PR，无动作
  - **已合且 tag 已打**：`git revert -m 1 <merge-sha>`；删 tag。单 change 单 merge commit，回退清晰可逆
  - **partial 失败**：按 sub-batch 顺序回退 (Stdlib → Runtime → Typechecker)，依赖箭头反向

## 待明确问题

- **`@bench` 属性同步支持？** roadmap.md §10.3 列了但 DP-1 未纳入，留待 v0.5.8 — 负责人：compiler team。
- **3.4 narrowing 是否覆盖 `dyn` 联合？** 当前 `inference.rs` 对 `dyn` 保守，扩到 `dyn` 需重做 type guard 的 dyn dispatch — 负责人：typechecker owner，v0.5.7 落地后复盘。
