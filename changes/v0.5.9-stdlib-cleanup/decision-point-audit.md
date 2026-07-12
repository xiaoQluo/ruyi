# Decision-Point Audit Report

**变更**: v0.5.9-stdlib-cleanup
**生成时间**: 2026-07-12T19:35:00Z
**当前状态**: specifying
**Workflow**: full
**Branch**: dev/v0.5.9-stdlib-cleanup (not yet created)

## 汇总表

| DP | 名称 | 结果 | 时间戳 |
|----|------|------|--------|
| DP-0 | 用户确认门禁 | confirmed | 2026-07-12T19:00:00Z |
| DP-1 | 需求确认 | approved | 2026-07-12T19:05:00Z |
| DP-2 | 工件审查 | approved | 2026-07-12T19:15:00Z |
| DP-3 | 契约批准 | approved | 2026-07-12T19:18:00Z |
| DP-4 | 执行模式选择 | approved | 2026-07-12T19:20:00Z |
| DP-5 | 调试升级 | pending | — |
| DP-6 | 验证结果 | pending | — |
| DP-7 | 归档确认 | pending | — |

**统计**: 5/8 已记录（DP-0 至 DP-4 已审核），DP-5/DP-6/DP-7 在 executing 阶段填入。

## 逐决策点说明

### DP-0: 用户确认门禁

- **结果**: confirmed
- **时间戳**: 2026-07-12T19:00:00Z
- **解读**: 决策点 DP-0 已记录为 "confirmed"。基于玉帝飞书圣裁"推进 v0.5.9 候选"，按已识别的 R1-R4 + R5 五候选立项。

### DP-1: 需求确认

- **结果**: approved
- **时间戳**: 2026-07-12T19:05:00Z
- **解读**: 5 sub-batch 单一 change 范围、Scope in/out 显式、Non-goals 锁定、Success criteria Full e2e 明确。Decomposition: 5 sub-batch 各自可独立 git revert。玉帝飞书直接圣裁 4 个关键决策（单 change vs 多、Full e2e vs Sub-Set、sub-batch 顺序 R1→R5→R2→R3→R4、R1 strategy 1 = lto/codegen-units、R5 一次性 35 全表）。

### DP-2: 工件审查

- **结果**: approved
- **时间戳**: 2026-07-12T19:15:00Z
- **解读**: 4 件规划主件 + 5 件 delta spec + .spec-superflow.yaml + execution-contract.md 全部齐全。8 Decisions 锁定（D1-D8），6 Risks 详记，5 Handoff Rules + 5 Escalation Rules 完整。

### DP-3: 契约批准

- **结果**: approved
- **时间戳**: 2026-07-12T19:18:00Z
- **解读**: contract locked; 5 sub-batch 顺序锁死 T1→T5; 7 acceptance criteria 硬约束; 7 out-of-scope 明确; 5 handoff rules 完整; R1 fallback 含 strategy 3 探源 + partial release 拆分双路径。

### DP-4: 执行模式选择

- **结果**: approved
- **时间戳**: 2026-07-12T19:20:00Z
- **解读**: full 模式 + SDD (Spec-Driven Development) 路径。SDD 选择原因: (1) 5 sub-batch 含 1 严格 fallback path, 需 SDD 结构化决策; (2) R5 重构 60+ FFI 接入, 需 TDD 每条; (3) R2 parser fix 易触发级联, 需 33-example oracle; (4) Full e2e 验收要求每 sub-batch 可独立验证。

### DP-5: 调试升级

- **结果**: pending
- **时间戳**: —
- **解读**: 待 T1-T5 全部 commit 后填入。预期结果: 5/5 sub-batch TDD pass, no rollback invoked.

### DP-6: 验证结果

- **结果**: pending
- **时间戳**: —
- **解读**: 待 T1-T5 后运行 Full e2e acceptance 7 criteria 全部通过。

### DP-7: 归档确认

- **结果**: pending
- **时间戳**: —
- **解读**: 待 Full e2e 全部 pass 后, git merge --no-ff dev/v0.5.9-stdlib-cleanup → main, git tag -a v0.5.9, git push origin main v0.5.9, 飞书发布卡。

## Known Risks (from design.md)

| ID | Severity | Mitigation |
|----|----------|------------|
| R1 | HIGH | lto/codegen-units change; strategy #3 (deep investigation) fallback if it fails; no Sub-Set e2e fallback per D3 |
| R2 | MEDIUM | 33-example test suite as oracle; TDD per grammar addition; git revert if regression |
| R3 | LOW | stdlib/fmt.ry has no direct `__string_replace_all` callers; verification via new fmt_demo.ry + 4 unit tests |
| R4 | LOW | snapshot diff is a structural guarantee; any new lint blocks merge |
| R5 | MEDIUM | 60+ FFI entries in a single table is high-blast-radius refactor; smoke-test via cargo test for every entry |
| R6 | LOW | LTO-disabled perf regression acceptable; future v0.5.10+ can revisit via rlib-only or multi-crate split |

## Deferred to v0.5.10+ (post v0.5.9)

- Pre-existing ruyi_runtime GC clippy warnings (52 errors / 32 warnings; v0.5.5 inheritance)
- `__string_replace_all_legacy` deletion (deferred to v0.6.0 after one release cycle of deprecation)
- `__io_*` / `__process_*` / `__path_*` symbol hygiene (separate change)
- ruyi_runtime multi-crate split (R1 strategy #2 — rejected in design.md D3)
- Full JSON spec parser (placeholder is sufficient; v0.6+)
- Generic trait integration (R3 deeper work — v0.6+)
- Other parser bugs (only the 3 in R2 scope)
- Performance optimization of `__string_replace_all` 8-arg implementation (current O(n*m) worst case)

## Spec Self-Review (per brainstorming skill)

### 1. Placeholder scan
- ❌ No "TBD" / "TODO" / incomplete sections found in proposal.md, design.md, tasks.md, execution-contract.md, 5 specs
- ✓ All sections fully specified

### 2. Internal consistency
- ✓ `proposal.md` What Changes section matches `design.md` D2 sub-batch ordering
- ✓ `tasks.md` T1-T5 sub-batches match `execution-contract.md` Task Batches table
- ✓ `execution-contract.md` 7 Acceptance Criteria map to `design.md` G1-G5 Goals
- ✓ 5 `specs/*.md` are independent (no cross-spec references that could be broken)
- ⚠️ `proposal.md` Scope table mentions "examples/fmt_demo.ry" created — matches `specs/04-fmt-ffi-8arg.md` REQ-5

### 3. Scope check
- ✓ Single change, 5 sub-batches: appropriate scope for 10-12 hours of work
- ✓ Each sub-batch is independent and atomic
- ✓ Total file count (8 source + 1 new file + 9 planning files) is manageable for a single change

### 4. Ambiguity check
- ✓ D1 single change vs multi-change: unambiguous (玉帝 chose单)
- ✓ D2 sub-batch order: unambiguous (玉帝 chose R1→R5→R2→R3→R4)
- ✓ D3 R1 strategy: unambiguous (玉帝 chose strategy 1 = lto + codegen-units)
- ✓ D4 R5 scope: unambiguous (玉帝 chose一次性 35 全表)
- ✓ D5 R5 table structure: `&'static [BuiltinDecl]` chosen, but Q2 in design.md asks about file location (new file vs inline) — open question, can be resolved during T2
- ✓ D6 R2 grammar additions: exactly 3, no ambiguity
- ✓ D7 R3 naming: `__string_replace_all_legacy` is the only choice (avoids symbol collision)
- ✓ D8 R4 verification: snapshot diff is the gate; "zero new lints" is well-defined

### Self-Review Conclusion

**Pass.** No placeholders, no internal contradictions, no scope issues, no critical ambiguities. Q1 (R1 strategy 3 time-box) and Q2 (BUILTINS file location) are minor open questions that can be resolved during implementation without re-review.

## Approvals Required (DP-5 → DP-7)

- **DP-5** (after T1-T5 commits): confirm no rollback was invoked
- **DP-6** (after T1-T5): confirm 7 acceptance criteria pass
- **DP-7** (after merge to main): confirm annotated tag v0.5.9 created, pushed, and 飞书发布卡 sent
