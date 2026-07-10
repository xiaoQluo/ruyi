# SDD Progress Ledger

**Change**: v0.5.5-residual-fixes
**Mode**: SDD (Spec-Driven Development)
**Started**: 2026-07-10
**Branch**: feature/v0.5.5-residual-fixes
**Worktree**: ../ruyi-v0.5.5-residual-fixes

## Execution Batches

### Batch 1.1: GC 双模式（4 任务）

| Task | Status | Commit | Review |
|------|--------|--------|--------|
| T-1.1.1 GcMode parse | done | e4830f0 | approved (DEV-001) |
| T-1.1.2 GcAllocFn dispatch | done | 2221a23 | approved (DEV-001) |
| T-1.1.3 CLI --gc flag | done | 43f5595 | approved (DEV-001) |
| T-1.1.4 codegen 全部堆分配切换 | done | ac92134 | approved (DEV-001) |

### Batch 1.2: 静态链接（2 任务）

| Task | Status | Commit | Review |
|------|--------|--------|--------|
| T-1.2.1 ruyi_runtime .a 产出 | pending | — | — |
| T-1.2.2 driver 链入 .a | pending | — | — |

### Batch 1.3: T9 收尾 + stdlib 审查（3 任务）

| Task | Status | Commit | Review |
|------|--------|--------|--------|
| T-1.3.1 RangeError/ArrayIterator 构造器 | done | 21028de | approved |
| T-1.3.2 启用 21 个 codegen #[ignore] | done | 3245ae2 | approved |
| T-1.3.3 stdlib audit 工具 + 报告 | done | (pending commit) | approved |
| T-1.3.3 stdlib audit 工具 + 报告 | pending | — | — |

### Batch 1.4: trait 约束检查（3 任务）

| Task | Status | Commit | Review |
|------|--------|--------|--------|
| T-1.4.1 ImplTable 数据结构 | done | e5a9662 | approved |
| T-1.4.2 check_bounds 实际验证 | done | 10ca3c7 | approved |
| T-1.4.3 启用 5+ typechecker #[ignore] | done | ae53aea | approved |

### Batch 2.1: ruyi_await 真实化（3 任务）

| Task | Status | Commit | Review |
|------|--------|--------|--------|
| T-2.1.1 Scheduler + Worker | pending | — | — |
| T-2.1.2 ruyi_await 真实实现 | pending | — | — |
| T-2.1.3 codegen 调用 ruyi_await | pending | — | — |

### Batch 2.2: try/catch landing pad（3 任务）

| Task | Status | Commit | Review |
|------|--------|--------|--------|
| T-2.2.1 CodegenContext.try_stack | pending | — | — |
| T-2.2.2 compile_try 完整 invoke | pending | — | — |
| T-2.2.3 启用 16 个 try/catch #[ignore] | pending | — | — |

### Batch 3: spawn 内建（3 任务）

| Task | Status | Commit | Review |
|------|--------|--------|--------|
| T-3.1 spawn builtin IR | pending | — | — |
| T-3.2 spawn_demo example | pending | — | — |
| T-3.3 spawn 集成测试 | pending | — | — |

### Batch 4: 验证与归档（2 任务）

| Task | Status | Commit | Review |
|------|--------|--------|--------|
| T-4.1 整体回归 + roadmap 更新 | pending | — | — |
| T-4.2 release-archivist 流程 | pending | — | — |

## Pre-conditions

- [x] DP-0 confirmed (v0.5.5-residual-fixes scope)
- [x] DP-1 confirmed (4 phases, 7 P0)
- [x] DP-2 confirmed (4 artifacts approved)
- [x] DP-3 confirmed (execution contract approved)
- [x] DP-4 confirmed (SDD mode)
- [x] v0.2-codegen-gaps archived to docs/archive/
- [x] worktree created at ../ruyi-v0.5.5-residual-fixes
- [ ] fix-try-catch-invoke archived (release-archivist pending)

## Per-Task Progress Notes

(每个 task 完成时追加 review summary、commit hash、任何 concerns)