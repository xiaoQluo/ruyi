# 异步任务 GC 根注册

## ADDED Requirements

### Requirement: 挂起任务栈帧加入 GC 根集合

`ruyi_runtime::gc::roots::register_async_roots` SHALL 遍历 `Scheduler::suspended_tasks()`，对每个挂起任务的 future 链调用 `GcVisitor::visit_root()`，使任务持有的堆引用被纳入 GC 根集合，结束空实现（no-op）行为。

#### Scenario: GC 触发时收集挂起任务根

- **WHEN** 用户代码调用 `ruyi_gc_collect()` 且当前存在挂起的 async 任务
- **THEN** `register_async_roots` MUST 在 mark 阶段前执行，挂起任务栈帧中的所有引用 MUST 被标记为根

#### Scenario: 调度器无可挂起任务时不报错

- **WHEN** `Scheduler::suspended_tasks()` 返回空
- **THEN** `register_async_roots` MUST 立即返回，不得触发额外分配或 panic

### Requirement: 任务持有引用的对象存活跨次 GC

任意挂起任务对某 GC 对象的引用 MUST 使该对象在后续 `ruyi_gc_collect` 后仍可达，不被回收。

#### Scenario: 1000 任务 1000 对象全部保留

- **WHEN** 测试代码分配 1000 个对象、`spawn` 10 个挂起任务各持 100 个对象引用，随后调用 `ruyi_gc_collect()`
- **THEN** 所有 1000 个对象 MUST 在 GC 后仍可达，集成测试 `async_gc_roots.rs` 全部断言通过

#### Scenario: 多层 future 链引用穿透

- **WHEN** 挂起任务通过 `await` 嵌套传递对象引用（深度 ≥ 3）
- **THEN** GC MUST 沿 future 链递归 mark 所有可达对象，不得因 future 状态机内部结构而漏标记

### Requirement: 任务结束后引用不再延长对象生命周期

async 任务执行完毕（返回或 panic）后，任务局部持有的对象引用 MUST 不再作为 GC 根，对象在下次 GC 后可被回收。

#### Scenario: 已完成任务不延长对象生命周期

- **WHEN** 任务 `t` 已完成且调度器已清理其 future 帧，任务曾持有的对象 `o` 仅由 `t` 单点引用
- **THEN** 调用 `ruyi_gc_collect()` 后 `o` MUST 被回收，`o.is_alive()` 返回 `false`

#### Scenario: 仍存活根引用保留对象

- **WHEN** 任务 `t` 已完成，但栈上其他活跃帧仍引用 `o`
- **THEN** `o` MUST 在 GC 后仍可达，证明仅任务结束的局部引用被解除