# Execution Contract: v0.5.9-stdlib-cleanup

**Change**: `v0.5.9-stdlib-cleanup`
**Mode**: full
**State**: specifying
**Branch**: `dev/v0.5.9-stdlib-cleanup` (not yet created)

## Intent Lock

Close all 4 known risks from `v0.5.8-stdlib-core/decision-point-audit.md` (R1 archive anomaly, R2 parser `dyn`/`?:` bugs, R3 fmt_ffi 8-arg migration, R4 zero-new-clippy) plus a 5th refactor (R5 table-driven codegen) in a single release with **full end-to-end acceptance** (33/33 examples pass, all 9 stdlib `.ry` files `--check` pass, zero new clippy lints, `make run-example` produces running binaries for `math_demo` and a new `fmt_demo`).

## Affected Scope

### Source code (8 files, 5 sub-batches)

| Sub-batch | File | Change |
|-----------|------|--------|
| R1 | `Cargo.toml` | Release profile: `lto=false`, `codegen-units=16` |
| R5 | `crates/ruyic/src/codegen/builtins_table.rs` | **Create**: `BuiltinSig` + `BuiltinDecl` + `BUILTINS` static slice |
| R5 | `crates/ruyic/src/codegen/builtins.rs` | Modify: replace 60+ `fn declare_*` with `declare_builtins` iterating `BUILTINS` |
| R5 | `crates/ruyic/src/typechecker/inference.rs` | Modify: `resolve_builtin_name` walks `BUILTINS` |
| R2 | `crates/ruyic/src/parser/parser.rs` | Modify: 3 grammar productions (dyn return, dyn param, `?:`) |
| R3 | `crates/ruyi_runtime/src/builtins.rs` | Modify: `__string_replace_all` (3-arg) → `__string_replace_all_legacy` |
| R3 | `crates/ruyi_runtime/src/fmt_ffi.rs` | Modify: `ruyi_string_replace_all` (8-arg) → `__string_replace_all` |
| R3 | `examples/fmt_demo.ry` | **Create**: R3 verification example |

### Planning artifacts

- `changes/v0.5.9-stdlib-cleanup/proposal.md`
- `changes/v0.5.9-stdlib-cleanup/design.md`
- `changes/v0.5.9-stdlib-cleanup/tasks.md`
- `changes/v0.5.9-stdlib-cleanup/execution-contract.md` (this file)
- `changes/v0.5.9-stdlib-cleanup/.spec-superflow.yaml`
- `changes/v0.5.9-stdlib-cleanup/decision-point-audit.md`
- `changes/v0.5.9-stdlib-cleanup/specs/01-archive-anomaly.md` (R1)
- `changes/v0.5.9-stdlib-cleanup/specs/02-codegen-table-driven.md` (R5)
- `changes/v0.5.9-stdlib-cleanup/specs/03-parser-dyn-optional.md` (R2)
- `changes/v0.5.9-stdlib-cleanup/specs/04-fmt-ffi-8arg.md` (R3)
- `changes/v0.5.9-stdlib-cleanup/specs/05-clippy-verify.md` (R4)

## Task Batches (5 sub-batches, in order)

| ID | Sub-batch | Files | LOC est. | TDD pattern |
|----|-----------|-------|----------|-------------|
| T1 | R1 archive | `Cargo.toml` | 2 | Write + `cargo build --release` + `ar t ... \| grep` |
| T2 | R5 codegen table | 3 files | ~250 | Write table → iterate → `cargo build --release` → `ruyic --check` 9 stdlib files |
| T3 | R2 parser | 1 file | ~30 | Write fix → `ruyic --check stdlib/{json,random}.ry` + 33-example regression |
| T4 | R3 fmt_ffi | 3 files | ~15 | Rename → `cargo build --release` → `make run-example EXAMPLE=fmt_demo` |
| T5 | R4 clippy verify | 0 (verify-only) | 0 | Snapshot diff + fix any new lints |

**Total estimated time**: 10-12 hours of focused work.

## Approved Behavior

After merge to main and tag `v0.5.9`:

1. `target/release/libruyi_runtime.a` (after `cargo clean -p ruyi_runtime && cargo build --release`) contains `math_ffi.o`, `time_ffi.o`, `json_ffi.o` and the `__math_*` / `__time_*` / `__json_*` symbols
2. `make run-example EXAMPLE=math_demo` runs and prints expected values (PI ≈ 3.14159, sqrt(16) = 4, etc.)
3. `make run-example EXAMPLE=fmt_demo` runs and demonstrates `__string_replace_all` 8-arg
4. `bash examples/run_examples.sh` exits 0 with 33/33 examples passing
5. `ruyic --check stdlib/{core,string,io,error,option,result,process,path,collections,math,time,json,random}.ry` all return "Type checking passed."
6. `cargo clippy --workspace` output minus v0.5.8 baseline is empty (no new lints)

## Acceptance Criteria

```bash
# 1. Workspace check
cargo check --workspace                                          → exit 0

# 2. Release build
cargo build --release                                            → exit 0

# 3. Tests
cargo test --workspace                                           → ≥ 110 tests pass

# 4. Stdlib e2e (all 9 files)
for f in core string io error option result process path collections math time json random; do
  ./target/release/ruyic --check "stdlib/$f.ry"
done
# All → "Type checking passed."

# 5. Examples
bash examples/run_examples.sh                                    → 33/33 PASS

# 6. Binary e2e (R1 verification)
make run-example EXAMPLE=math_demo                                → runs, prints expected
make run-example EXAMPLE=fmt_demo                                 → runs, prints expected

# 7. Archive anomaly fixed
ar t target/release/libruyi_runtime.a | grep -cE 'math_ffi|time_ffi|json_ffi'   # ≥ 3
nm target/release/libruyi_runtime.a | grep -cE '__math_abs|__time_now|__json_parse'  # ≥ 3

# 8. Clippy zero-new (R4)
diff <(cargo clippy --workspace 2>&1 on v0.5.8 | sort) \
     <(cargo clippy --workspace 2>&1 on v0.5.9 | sort)   # empty diff

# 9. Format
cargo fmt --check                                                → exit 0
```

## Out of Scope (Scope Fence)

- ❌ pre-existing ruyi_runtime GC clippy warnings (R4 forbids *new* lints, not their existence)
- ❌ ruyi_runtime multi-crate split (R1 strategy #2 — rejected in design.md D3)
- ❌ full JSON spec parser (placeholder sufficient; v0.6+)
- ❌ `__io_*` / `__process_*` / `__path_*` symbol hygiene (separate change)
- ❌ `__string_replace_all_legacy` deletion (deferred to v0.6.0 after one release cycle)
- ❌ generic trait integration (R3 deeper work — v0.6+)
- ❌ other parser bugs (only the 3 in R2 scope)
- ❌ stdlib feature expansion beyond `dyn` and `?:` semantics

## Handoff Rules

- Sub-batches proceed in fixed order: **T1 → T2 → T3 → T4 → T5**
- Each sub-batch is its own commit on `dev/v0.5.9-stdlib-cleanup` for atomic rollback
- If T1 fails: R1 strategy #3 fallback (no Sub-Set e2e fallback per D3)
- If T2 breaks existing FFI behavior: revert to T1 + cherry-pick T1 only + mark R5 as v0.5.10 follow-up
- If T3 breaks existing examples: git revert T3 commit + keep R2 as v0.5.10 follow-up
- If T4 breaks `stdlib/fmt.ry`: git revert T4 + keep R3 as v0.5.10 follow-up
- If T5 finds > 5 new lints: defer T5 to v0.5.10 follow-up; v0.5.9 ships with T1-T4

## Risks

See `design.md` Risks section for full list (R1 archive, R2 parser cascade, R3 fmt_ffi migration, R4 clippy, R5 codegen ABI, R6 LTO perf).

| ID | Severity | Mitigation |
|----|----------|------------|
| R1 | HIGH | lto/codegen-units change; strategy #3 fallback if it fails |
| R2 | MEDIUM | 33-example test suite as oracle; TDD per grammar addition |
| R3 | LOW | `stdlib/fmt.ry` has no direct `__string_replace_all` callers |
| R4 | LOW | snapshot diff is a structural guarantee; any new lint blocks merge |
| R5 | MEDIUM | 60+ FFI entries in a single table is a high-blast-radius refactor; smoke-test via `cargo test -p ruyi_runtime --lib` for every entry |

## Escalation Rules

1. **T1 fails after 30 min**: escalate to R1 strategy #3 (deep investigation). If strategy #3 also fails, **split v0.5.9** — commit T2 + T3 + T4 + T5 as a partial `v0.5.9-partial` and defer T1 to `v0.5.10`. Per D3: **no Sub-Set e2e fallback**.
2. **T2 breaks existing FFI behavior**: revert T2 commit, ship `v0.5.9-codegen-table` as a separate change (T1 only). Per design.md: codegen refactor is too high-blast-radius to risk.
3. **T3 or T4 cause > 1 example regression**: git revert that sub-batch, ship partial.
4. **T5 finds > 5 new lints**: defer to v0.5.10.
5. **Multiple sub-batches fail simultaneously**: STOP, return to specifying state, re-evaluate scope.

## Approval Gate (DP-3) → (DP-7) Sequence

| DP | Action | State transition |
|----|--------|------------------|
| DP-3 | Execution-contract approved by 玉帝 (this doc) | specifying → bridging |
| DP-4 | Execution mode selected: SDD (Spec-Driven Development) | bridging → approved-for-build |
| DP-5 | Debug escalation: R1 strategy #3 fallback available if T1 fails | approved-for-build → executing |
| DP-6 | 5-dim verification (per design.md) | executing → closing |
| DP-7 | Archive closure: tag `v0.5.9` (annotated), `git push origin main v0.5.9`, lark card | closing → closed |

## Notes for Implementation

- **Pre-flight before T1**: `git checkout -b dev/v0.5.9-stdlib-cleanup` from current main (`1acd1d6`)
- **Pre-flight before T2**: ensure T1 is committed (R2 and beyond are tested via `cargo build --release` which depends on the archive anomaly being fixed)
- **T5 must be the last sub-batch**: do not run clippy verification until T1-T4 are all committed
