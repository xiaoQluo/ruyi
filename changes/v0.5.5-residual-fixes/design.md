# Design: v0.5.5-residual-fixes

## Context

### Current State

Ruyi v0.5.5 (`dev/v0.5.5` 分支) 存在 7 项 P0 缺陷，阻塞编译器发布：

1. **GC 占位**：codegen 中所有堆分配走裸 `malloc`（`cc_alloc`），`ruyi_runtime/gc/` 已有 generational GC 框架但未连通
2. **运行时库未链入**：`driver.rs` 直接调 `cc`，未链 `libruyi_runtime.a`，导致 `ruyi_await`、`spawn`、try/catch landing pad 均无法真正工作
3. **T9 收尾未完成**：commit `809e6c9` 让 `RangeError` / `ArrayIterator` 成为 Named 类型，但未实现为可构造器调用；致 21/27 codegen 测试 FAIL
4. **try/catch 部分落地**：`fix-try-catch-invoke` (state: closing) 修了 `compile_try` 使用 `build_invoke`，但 fix 仅在本函数内有效，跨函数 catch 仍 fail
5. **trait 约束检查为空**：`generics.rs::check_bounds()` 始终返回 `true`，致 32 个 typechecker 测试被 `#[ignore]`
6. **ruyi_await 空操作**：`async_codegen.rs` 中 `ruyi_await` 是 stub
7. **spawn 内建未实现**：调度器存在但入口未暴露

### Constraints

- **LLVM 14 必需**：inkwell 绑定要求；本机已装（`LLVM_SYS_140_PREFIX`）
- **Rust 2021 edition** + clippy zero-warning 原则（不可妥协）
- **静态库必须可独立构建**：`cargo build -p ruyi_runtime --release` 必须产出 `libruyi_runtime.a`
- **不影响现有 examples**：33/33 行为不变
- **Javadoc 注释**保留：`@author`, `@date` 必须存在
- **修改必须从方案完整性和合理性出发**：不留技术债

### Stakeholders

- **编译器用户**：写 .ry 程序的开发者，关心能否真正用 async/await、try/catch、GC
- **库作者**：写 stdlib 的开发者，关心 RangeError 等可构造器
- **编译器维护者**：关心代码可读、可维护、零警告

## Goals

1. **解锁 21 个 FAIL codegen 测试**：完成 T9 收尾
2. **静态链入 ruyi_runtime**：单文件可执行二进制
3. **双模式 GC**：`--gc=stub` 保持开发速度；`--gc=real` 启用真实 GC
4. **真实化 ruyi_await**：async/await 真正能跑
5. **try/catch 跨函数异常捕获**：landing pad 完整落地
6. **trait 约束实际验证**：泛型正确性保证
7. **spawn 内建**：green thread 启动入口
8. **4 阶段交付**：每阶段可独立验证、可回滚
9. **零警告、零技术债**：clippy clean，无 TODO 残留

## Decisions

### Decision 1: GC 双模式用 `--gc=stub/real` 编译时切换

**Choice**: 编译时 flag 切换（`--gc=stub` / `--gc=real`），codegen 在每个堆分配点检查 flag 后选择调用 `cc_alloc` 或 `ruyi_gc_alloc`。

**Rationale**:
- 编译时切换零运行时开销（无 if 分支）
- 与现有 codegen 流程兼容（无需新增运行时调度）
- stub 模式保留开发速度（无 GC 编译时间 +30% 开销）
- 用户按需启用真实 GC

**Alternatives considered**:
- 运行时切换：在 `ruyi_gc_alloc` 内部 stub/real 切换 → 增加二进制体积 + 函数调用开销，否决
- 只支持 real 模式：直接全部启用 → 开发体验恶化（编译时间 +30%），否决
- 自动探测：编译器根据 `--opt-level` 自动选 → 行为不透明，难以调试，否决

### Decision 2: 静态链接用 `cc-rs` crate 集成 `libruyi_runtime.a`

**Choice**: 使用 `cc` crate（Rust 标准 C/C++ 编译工具）将 `libruyi_runtime.a` 链入目标二进制，链接命令通过 `cc::Build` 构造。

**Rationale**:
- `cc` crate 已是 Rust 生态标准，跨平台（Linux/macOS/Windows）
- 与现有 `driver.rs` 中 `cc::Build` 调用风格一致
- 自动处理平台差异（`.a` / `.lib`、链接顺序）

**Alternatives considered**:
- 直接调 `Command::new("cc")`：脆弱、跨平台差，否决
- 用 `rustc --extern`：要求 runtime crate 是 Rust-only 且与 codegen 兼容，不适用（runtime 含 C 汇编），否决
- 动态链接 `.so/.dylib`：破坏"单文件可执行"用户预期，否决

### Decision 3: T9 收尾通过 stdlib 类构造函数实现

**Choice**: 在 `stdlib/collections.ry` 中将 `RangeError` / `ArrayIterator` 的构造函数补全（接受 `message` / `array` 参数），与用户类构造函数语义一致。

**Rationale**:
- 与 Ruyi 用户类构造模式一致（`new MyClass(args)`）
- 类型检查器只需识别"类类型可构造"通用规则，无需为 stdlib 特判
- 修改局部、风险可控

**Alternatives considered**:
- 在类型检查器中维护"named constructible types"白名单：硬编码、不可扩展，否决
- 把 RangeError/ArrayIterator 从 Named 类型改为 trait type：影响类型系统语义，否决
- 在 codegen 特殊路径处理：破坏抽象层，否决

### Decision 4: trait 约束检查用 HashMap<ImplKey, ImplDef>

**Choice**: `generics.rs` 维护一个全局 `impl_table: HashMap<(TraitId, TypeId), ImplDef>`，由 trait/impl 注册时填充，`check_bounds` 查询此表。

**Rationale**:
- O(1) 查询，编译期高效
- 与现有类型系统数据结构兼容
- 支持独立 `impl Trait for Type` 块（REQ-TRAIT-003）

**Alternatives considered**:
- AST 全遍历：O(N) 每次约束检查，编译慢，否决
- 类型推导阶段查询：与"trait bound 检查"职责混淆，否决
- 单独的 trait solver crate（如 chalk）：引入大依赖，与渐进式发布不符，否决

### Decision 5: ruyi_await 用 stackless coroutine + Future trait

**Choice**: `Future` trait 由 `poll(self) -> Poll<T>` 组成；`ruyi_await(future)` 调用 `poll`，未就绪则将当前 coroutine 挂起到 scheduler ready queue。

**Rationale**:
- Stackless 协程轻量（coroutine 状态 ≈ Future 字段），适合 green thread 场景
- 与 `spawn` 共享调度器基础设施
- 实现简单，无需栈切换

**Alternatives considered**:
- Stackful coroutine (如 `tokio::task)：重量级，需保存完整栈，与 Ruyi "原生机器码 + LLVM" 定位不符，否决
- 回调式（callback-based）：与 async/await 语法糖不匹配，用户体验差，否决
- 生成器（generator）状态机：与 LLVM IR 集成复杂，否决

### Decision 6: 工作窃取调度器用 `crossbeam-deque` 作为底层

**Choice**: 调度器每个 worker 持有 `crossbeam_deque::Worker`，调度入口用 `Injector`；worker 间用 `Stealer` 窃取。

**Rationale**:
- `crossbeam-deque` 是 Rust 生态成熟的无锁 work-stealing deque
- 性能经过实战验证（Tokio 等采用）
- 减少自研风险

**Alternatives considered**:
- 手写无锁队列：风险高、需大量测试，否决
- 单线程调度：不能利用多核，与"工作窃取"目标矛盾，否决
- 第三方调度器（如 Tokio）：依赖过大，与 Ruyi "自研运行时"定位不符，否决

### Decision 7: spawn fire-and-forget（无 JoinHandle）

**Choice**: `spawn(fn)` 启动后立即返回 `void`，不提供 `await handle` 机制；用户如需等待结果，使用 `await future` 在 future 内部实现 join。

**Rationale**:
- 与 Ruyi 现有 Future 模型一致（future 可内部 join）
- 简化 builtin API（无需新增 `JoinHandle` 类型）
- 用户在大多数场景不需要 join

**Alternatives considered**:
- `spawn` 返回 `JoinHandle<T>`：需新增类型，与现有 stdlib 不兼容，推迟到后续 change
- `spawn` 是 macro 不是 builtin：复杂度高，否决

### Decision 8: stdlib 8 模块审查作为 #2 子任务，与 T9 收尾并行

**Choice**: 阶段 1 的 #2 P0 包含两项工作并行：
- T9 收尾（RangeError/ArrayIterator 构造器化）—— 必需
- stdlib 8 模块正确性审查 —— 输出审计报告，**不实装** math/time/json

**Rationale**:
- 复用 stdlib合理性检查 探索成果
- 审查报告作为后续 change 的输入
- 不蔓延范围（math/time/json 实装留后续）

**Alternatives considered**:
- stdlib 审查单独一个 change：增加状态机负担，否决
- stdlib 审查完全不做：失去探索成果，否决

## Risks And Trade-Offs

| 风险 | 等级 | 缓解策略 |
|------|------|---------|
| **静态链接二进制膨胀** | 中 | 后续 change 评估 strip / LTO；当前 +1MB 在可接受范围 |
| **GC 真实模式编译时间 +30%** | 中 | stub 默认；real 显式 opt-in |
| **工作窃取调度器并发 bug** | 高 | 阶段 2 单独迭代；用 `loom` 做并发测试 |
| **T9 修复后其他类型（如 Map/Set）也需构造器化** | 中 | 阶段 1 收尾时一并扫，输出后续 change 清单 |
| **trait 约束检查回归（泛型代码误报）** | 高 | 32 个 #[ignore] 测试分批启用，先 5 个观察回归 |
| **async/await 实际性能不达预期** | 低 | 阶段 2 单独基准测试（criterion） |
| **driver.rs 改动牵涉 linker 平台差异** | 中 | CI 多平台（Linux + macOS）验证 |
| **跨函数 try/catch 修复与 fix-try-catch-invoke 冲突** | 中 | 阶段 2 启动前先归档 fix-try-catch-invoke，明确基线 |
| **stdlib 审查未发现深层次问题** | 低 | 审查仅输出报告，不阻塞 P0 修复 |

## Validation Checklist

- [x] `## Context` — 当前状态、约束、利益相关者
- [x] `## Goals` — 9 项
- [x] `## Decisions` — 8 项（每项含 Choice + Rationale + Alternatives）
- [x] `## Risks And Trade-Offs` — 9 项风险与缓解

完整文卷已存于 `changes/v0.5.5-residual-fixes/design.md`。