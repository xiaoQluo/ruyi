# SDD Progress Ledger: fix-try-catch-invoke

**Change**: fix-try-catch-invoke
**State**: closing
**Workflow**: full → SDD (DP-4 approved)
**Branch**: dev/v0.5.5

## Batches

- [x] Batch 1: 基础设施(2 项并行)
  - [x] T1: ruyi_exception shared crate + LandingPadGenerator 迁移
  - [x] T2: CodegenContext.try_stack + TryStackGuard
- [x] Batch 2: 核心改造(2 项并行,依赖 Batch 1)
  - [x] T3: compile_throw unreachable
  - [x] T4: compile_try build_invoke + landingpad
- [x] Batch 3: 调用方改造(1 项,依赖 Batch 2)
  - [x] T5: compile_call 感知 try 上下文
- [x] Batch 4: 验证、新 example 与文档(3 项并行)
  - [x] T6: examples/try_catch_invoke.ry + run_examples.sh
  - [x] T7: codegen 集成测试(#[ignore])
  - [x] T8: TRY_CATCH_AUDIT.md §3 + §5
- [x] FIX1: `LandingPadGenerator::get_type_info_global` Internal linkage + null initializer (修复 undefined symbol 链接错误)
- [x] FIX2: `build_catch_dispatch` 简化为 catch-all + `compile_throw` 非 try 分支 emit `return <zero>` (修复 SIGILL)

## Per-Task Progress

(commits will be appended here as each task completes)

## Status

**Final state**: closing (DP-7 archived)

**Verification**:
- 134 → 133 lib tests pass
- 34/34 examples pass (including new try_catch_invoke.ry)
- 3/3 ruyi_exception tests pass
- 0 cargo warnings
- 0 clippy warnings
- codegen integration tests `#[ignore]` ready

**Known issues (out of scope)**:
- 5 diagnostic tests fail (separate change)
- CI disabled (separate change)
- v0.5.5 release tag/merge (separate change)
