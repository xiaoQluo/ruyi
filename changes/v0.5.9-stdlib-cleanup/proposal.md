# Proposal: v0.5.9-stdlib-cleanup

## Why

`v0.5.8-stdlib-core` (commit `cb8822e` + `a5bfd73`, merged to main as `1acd1d6`, tag `v0.5.8`) shipped 20 new FFI entries (`__math_*` / `__time_*` / `__json_*`) across the compiler's three-layer FFI surface. The release was a **source-layer success** but `decision-point-audit.md` recorded four known risks (R1–R4) deferred to a follow-up change:

- **R1** — runtime archive anomaly: `math_ffi.o` / `time_ffi.o` / `json_ffi.o` are not packed into `target/release/libruyi_runtime.a` (215 MB archive contains only LLVM `AArch64A*.o` and `X86*.o` objects)
- **R2** — parser bugs: `fn f(): dyn { ... }` and `parseInt(s: int? = 0)` fail at parse time, blocking `stdlib/json.ry` and `stdlib/random.ry` end-to-end
- **R3** — fmt_ffi 8-arg migration: `fmt_ffi.rs` was reverted in v0.5.8 due to a name collision with `builtins.rs:773`'s 3-arg `__string_replace_all`
- **R4** — pre-existing ruyi_runtime GC clippy warnings (52 errors / 32 warnings) need to not be added to by v0.5.9

A fifth follow-up, **R5 — table-driven codegen**, was proposed in the v0.5.8 wrap-up as "the way a good developer improves code they're working in": 60+ hand-written `fn declare_*` in `codegen/builtins.rs` (1100 lines) are repetitive boilerplate that any new FFI would extend.

`v0.5.9-stdlib-cleanup` closes all 5 in a single change with full end-to-end acceptance.

## What Changes

### Sub-batch 1 (R1) — runtime archive anomaly
Edit `[profile.release]` in workspace `Cargo.toml`: `lto = true` → `lto = false`, `codegen-units = 1` → `codegen-units = 16`. Result: `target/release/libruyi_runtime.a` contains `math_ffi.o` / `time_ffi.o` / `json_ffi.o`; `make run-example EXAMPLE=math_demo` produces a running binary that prints expected math values.

### Sub-batch 2 (R5) — table-driven codegen
Replace 60+ hand-written `fn declare_*(context, module)` in `codegen/builtins.rs` with a single `pub static BUILTINS: &[BuiltinDecl]` table iterated at module-load time. Same 35 FFI entries; same LLVM ABI; same typechecker signatures. All five `pub use crate::*_ffi::{...}` re-exports unchanged. `codegen/builtins_table.rs` (new file) holds the table.

### Sub-batch 3 (R2) — parser fixes
Add 3 grammar productions to `parser/parser.rs`:
1. `dyn` as return type (`fn f(): dyn { ... }`)
2. `dyn` as parameter type (`(x: dyn)`)
3. `?:` optional parameter syntax (`(s: int? = default)`) with default value propagation through typechecker and codegen

### Sub-batch 4 (R3) — fmt_ffi 8-arg migration
- Rename `builtins.rs:773` `__string_replace_all` (3-arg, C string) → `__string_replace_all_legacy` (with deprecation comment)
- Rename `fmt_ffi.rs:53` `ruyi_string_replace_all` (8-arg, bounded) → `__string_replace_all` (A path)
- Update `codegen/builtins.rs` `declare_string_replace_all` to use 8-arg LLVM signature
- Verify `stdlib/fmt.ry` callers (if any) updated

### Sub-batch 5 (R4) — zero-new-clippy verification
Run `cargo clippy --workspace` on v0.5.9 and on v0.5.8 baseline; diff the lint sets. Acceptance: empty diff. Any new lint blocks the merge.

## Scope (in)

| File | Action |
|------|--------|
| `Cargo.toml` | Modify (release profile: lto + codegen-units) |
| `crates/ruyic/src/codegen/builtins.rs` | Modify (60+ `fn declare_*` → table dispatch) |
| `crates/ruyic/src/codegen/builtins_table.rs` | **Create** (static `BUILTINS` table + dispatch helpers) |
| `crates/ruyic/src/typechecker/inference.rs` | Modify (`resolve_builtin_name` walks `BUILTINS` table for the 35 FFI) |
| `crates/ruyic/src/parser/parser.rs` | Modify (3 grammar productions: dyn return, dyn param, `?:`) |
| `crates/ruyic/src/codegen/expr.rs` | Modify (default value propagation for optional params, if needed) |
| `crates/ruyi_runtime/src/builtins.rs` | Modify (`__string_replace_all` → `__string_replace_all_legacy`) |
| `crates/ruyi_runtime/src/fmt_ffi.rs` | Modify (`ruyi_string_replace_all` → `__string_replace_all`) |
| `examples/fmt_demo.ry` | **Create** (new example for R3 verification) |
| `changes/v0.5.9-stdlib-cleanup/` | **Create** (9 件法度文件) |

## Scope (out / Scope Fence)

- ❌ pre-existing ruyi_runtime GC clippy warnings (R4 forbids new lints, not their existence)
- ❌ ruyi_runtime multi-crate split (R1 strategy #2 — rejected)
- ❌ full JSON spec parser (placeholder is sufficient; v0.6+)
- ❌ `__io_*` / `__process_*` / `__path_*` symbol hygiene (separate change)
- ❌ stdlib feature expansion beyond `dyn` and `?:` (other parser bugs deferred)
- ❌ `__string_replace_all_legacy` deletion (deferred to v0.6.0 after one release cycle)
- ❌ generic trait integration (R3's deeper work — v0.6+)

## Impact

| Dimension | Impact |
|-----------|--------|
| Compiler binary size | +10-15% (LTO disabled) — acceptable for short-lived compiler processes |
| Compile time | faster (codegen-units=16 enables parallelism) |
| `make run-example` coverage | 33+ examples all pass; new `fmt_demo.ry` for R3 |
| `ruyic --check` coverage | all 9 stdlib `.ry` files pass end-to-end |
| `cargo test` coverage | 102 → ≥110 tests (add 4 `fmt_ffi` tests + 1-2 parser regression tests) |
| `cargo clippy` coverage | zero new lints (R4 snapshot diff) |
| ABI | `__string_replace_all` 3-arg deprecated (renamed to `_legacy`); 8-arg becomes the canonical |
| Public API | `pub use` re-exports unchanged; `BUILTINS` table is `pub` for typecheck access |

## Capabilities (CLOSED)

- `compiler-binary-e2e-validation`: `make run-example` produces running binaries
- `compiler-stdlib-parity`: 9/9 stdlib `.ry` files `--check` end-to-end pass
- `codegen-table-driven`: 35 FFI entries declared from a single static table
- `parser-dyn-grammar`: `dyn` as return / param type, `?:` optional param
- `fmt-ffi-8arg-canonical`: 8-arg bounded-buffer ABI is the only `__string_replace_all`
- `clippy-zero-new`: v0.5.9 diff vs v0.5.8 baseline clippy output is empty

## Acceptance

```bash
# T1
cargo clean -p ruyi_runtime && cargo build --release
ar t target/release/libruyi_runtime.a | grep -cE 'math_ffi|time_ffi|json_ffi'  # ≥ 3
make run-example EXAMPLE=math_demo                                           # runs, prints PI/sqrt/abs

# T2 (after T1)
cargo test -p ruyi_runtime --lib                                           # ≥ 110 tests pass
ruyic --check stdlib/collections.ry stdlib/string.ry stdlib/io.ry          # pass

# T3
ruyic --check stdlib/json.ry                                                # "Type checking passed."
ruyic --check stdlib/random.ry                                              # "Type checking passed."

# T4
make run-example EXAMPLE=fmt_demo                                            # runs
cargo test -p ruyi_runtime --lib fmt_ffi::                                   # 4 tests pass

# T5
diff <(cargo clippy --workspace 2>&1 | grep -cE "warning|error" on v0.5.8) \
     <(cargo clippy --workspace 2>&1 | grep -cE "warning|error" on v0.5.9)   # empty

# Final
bash examples/run_examples.sh                                                # 33/33 PASS
```
