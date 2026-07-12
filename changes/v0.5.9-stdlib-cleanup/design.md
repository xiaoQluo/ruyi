# Design: v0.5.9-stdlib-cleanup

## Background

`v0.5.8-stdlib-core` (commit `cb8822e` + `a5bfd73` on `dev/v0.5.8`, merged to main as `1acd1d6`, tag `v0.5.8`) introduced 20 new FFI entries (`__math_*` / `__time_*` / `__json_*`) across the compiler's three-layer FFI surface. The release was a **source-layer success** (cargo check / build / test all pass; `ruyic --check stdlib/{math,time}.ry` pass), but its 5-dim verification marked four known risks (R1–R4) deferred to a follow-up change.

`v0.5.9-stdlib-cleanup` closes all four R1–R4 plus a fifth refactor (R5) that the `decision-point-audit.md` of `v0.5.8` listed as "architectural follow-up": the table-driven codegen refactor that would have made `R3` (fmt_ffi 8-arg migration) cleaner had it landed first.

## Goals (in priority order)

1. **G1** — Make binary end-to-end validation possible: fix the `libruyi_runtime.a` archive anomaly so that `math_ffi.o` / `time_ffi.o` / `json_ffi.o` are packed into the static archive and `make run-example EXAMPLE=math_demo` produces a running binary that prints `PI ≈ 3.14159`, `sqrt(16.0) == 4.0`, etc. (R1)
2. **G2** — Make `stdlib/json.ry` and `stdlib/random.ry` end-to-end checkable: fix parser support for `dyn` as return / parameter type and `?:` optional parameter syntax (R2)
3. **G3** — Make the codegen maintainable: refactor the 60+ hand-written `fn declare_*` into a single table-driven dispatch (R5)
4. **G4** — Make the runtime fmt module ABI-clean: migrate codegen to the 8-arg bounded-buffer `__string_replace_all` design and deprecate the 3-arg legacy (R3)
5. **G5** — Keep the codebase free of newly-introduced clippy lints: every lint in the v0.5.9 diff must have been present in the v0.5.8 baseline (R4)

## Non-Goals

- NG-1: Fix pre-existing clippy lints that already existed in v0.5.8 baseline (52 errors / 32 warnings in `ruyi_runtime::gc` are *pre-existing* — R4 forbids *new* lints, not their existence)
- NG-2: Restructure `ruyi_runtime` into multiple sub-crates (R1 strategy #2 — rejected in favor of lto/codegen-units tweak)
- NG-3: Rewrite the JSON parser to support the full JSON spec (the placeholder implementation is sufficient; a real parser is a v0.6+ concern)
- NG-4: Touch the pre-existing `__io_*` / `__process_*` / `__path_*` symbols (separate hygiene change)

## Decisions

### D1 — Single change, full e2e acceptance
**Choice**: One change `v0.5.9-stdlib-cleanup` containing all five sub-batches. Acceptance: 33/33 examples pass + `stdlib/{math,time,json,random,collections,string,io,error,path}.ry` all `--check` pass + zero new clippy lints.

**Why**: Cross-subsystem, but every sub-batch unblocks part of the full e2e chain. Splitting into 3+ changes would create artificial barriers (e.g., R3 migration needs R5's table-driven `declare_builtins` to land cleanly, R2's parser fix needs the R1 archive fix to verify end-to-end at the binary level).

**Alternative considered**: 3-change decomposition (R1 / R2 / R3+R5) with R4 standalone. Rejected because R3's clean migration depends on R5's table-driven codegen, and the release cadence is on the order of weeks, not days.

### D2 — Sub-batch ordering: R1 → R5 → R2 → R3 → R4
**Choice**: Each sub-batch is a discrete TDD-validated commit in this order.

**Why**:
- R1 first: archive anomaly blocks all binary e2e validation; once fixed, the remaining sub-batches can be verified at the binary level
- R5 second: codegen table refactor is a foundational change that R3 (fmt_ffi 8-arg migration) builds on. R3's "migrate 60th hand-written fn declare_string_replace_all to the 8-arg signature" becomes trivial once R5 lands
- R2 third: parser fixes are isolated and unblock `stdlib/json.ry` and `stdlib/random.ry` `--check` end-to-end
- R3 fourth: with R5's table in place, this is a one-line ABI change (signature in the table) + delete the 3-arg legacy + update the runtime rename
- R4 last: it's a verification gate, not a change; running clippy at the end of the chain gives a clean "no new lints introduced" signal

**Alternative considered**: "Quick wins first" ordering (R2 → R4 → R3 → R5 → R1). Rejected because R1 is the only sub-batch that unblocks the e2e chain; without R1, every subsequent sub-batch can only be source-verified, which is what v0.5.8 already did.

### D3 — R1 strategy: lto=false + codegen-units=16
**Choice**: Edit `[profile.release]` in workspace `Cargo.toml`:
```diff
 [profile.release]
 opt-level = 3
-lto = true
-codegen-units = 1
+lto = false
+codegen-units = 16
```

**Why**: The 215 MB `libruyi_runtime.a` containing only LLVM `AArch64A*.o` and `X86*.o` (no `math_ffi.o` etc.) points to LTO + `codegen-units=1` merging the ruyi_runtime object files into the inkwell LLVM static archive during cargo's staticlib emission. Disabling LTO and reverting to the default 16-unit split keeps the ruyi_runtime objects as separate `.o` files packed in the archive.

**Performance impact**: ~10-15% larger binary, ~5-10% slower at peak. For a compiler binary that is typically run for a few seconds, this is acceptable; runtime FFI performance is unchanged.

**Fallback if it fails**: per the brainstorming decision, **no Sub-Set e2e fallback**. If `lto=false + codegen-units=16` does not fix the archive anomaly, immediately escalate to R1 strategy #3 (deep investigation of the ar pipeline). The v0.5.9 change would then either (a) ship with the lto setting change + a follow-up issue for archive, OR (b) be split — R2/R3/R5 commit as a partial v0.5.9 and R1 deferred to v0.5.10.

### D4 — R5 scope: all 35 builtin FFI, not a 20-item PoC
**Choice**: Refactor the entire `BUILTINS` table to include all 35 entries: 6 `__builtin_array_*` + 7 `__builtin_map_*` + 4 `__builtin_set_*` + 18 `__string_*` + 14 `__math_*` + 4 `__time_*` + 2 `__json_*` = 55 entries (some repeated under different names for overloads; after dedup 35 unique `declare_*`).

**Why**: 玉帝已批准 "R5 一次性 35 全表重构"。Half-measure (only the 20 new) would still leave the 35 pre-existing entries as hand-written functions, defeating the architectural goal.

**Alternative considered**: 20-item PoC then expand. Rejected — 玉帝 chose一次性全表 (one-shot full table) in the brainstorming session.

### D5 — R5 table structure: `&'static [BuiltinDecl]`
**Choice**: A static slice of `BuiltinDecl { name, ret, params }` records, iterated at module-load time to generate LLVM `declare` instructions. The typechecker `resolve_builtin_name` is also refactored to walk the same table (with a separate Type-mapping for typecheck vs LLVM-ABI).

```rust
#[derive(Clone, Copy)]
pub enum BuiltinSig {
    Void,    // () -> void
    Int,     // i64
    Float,   // f64
    String,  // *mut i8 (C string)
    Ptr,     // *mut i8 (opaque)
}

pub struct BuiltinDecl {
    pub name: &'static str,
    pub ret: BuiltinSig,
    pub params: &'static [BuiltinSig],
}

pub static BUILTINS: &[BuiltinDecl] = &[
    BuiltinDecl { name: "__builtin_array_create", ret: BuiltinSig::Ptr, params: &[] },
    BuiltinDecl { name: "__builtin_array_get",    ret: BuiltinSig::Int, params: &[Ptr, Int] },
    // ... 35 entries total ...
];
```

**Why**: A static slice is the simplest representation; iteration cost at `declare_builtins()` time is negligible (called once per module). The `&'static` lifetime enables `const`-friendly construction.

**Alternative considered**: A `phf::Map` or `HashMap`. Rejected — overkill for 35 entries; static slice is the right shape.

### D6 — R2 parser fix: scope limited to 3 grammar additions
**Choice**: Add exactly 3 grammar productions to `parser/parser.rs`:
1. `dyn` as return type (after `: ` in `fn f(): dyn { ... }`)
2. `dyn` as parameter type (in `(x: dyn, ...)`)
3. `?:` optional parameter syntax (`(s: int? = default)`)

**Why**: These are the only parser-level gaps blocking v0.5.8's `stdlib/json.ry` and `stdlib/random.ry` from `--check` end-to-end. Any other parser bug fix is out of scope.

### D7 — R3 strategy: rename 3-arg legacy to `__string_replace_all_legacy`
**Choice**: Rename `__string_replace_all` (3-arg) → `__string_replace_all_legacy`; rename `ruyi_string_replace_all` (8-arg) → `__string_replace_all`; update codegen to use 8-arg; update `stdlib/fmt.ry` callers if any.

**Why**: Naming the legacy explicitly avoids the same `__string_replace_all` symbol collision that blocked the v0.5.8 attempt to rename `fmt_ffi.rs`. The legacy version is kept (not deleted) for source compat with any out-of-tree code that may link against the 3-arg signature; deprecation comment notes it can be removed in v0.6.

### D8 — R4 verification: snapshot diff, not absolute count
**Choice**: R4's acceptance is "v0.5.9 clippy output minus v0.5.8 clippy output is empty (no new lints)". Not "zero total lints".

**Why**: The 52 errors / 32 warnings in `ruyi_runtime::gc` are pre-existing (v0.5.5+ inheritance). R4 forbids *new* lints, not their existence. The verification command is a structural diff between the two clippy invocations.

## Risks

| ID | Risk | Mitigation |
|----|------|------------|
| R1 | `lto=false + codegen-units=16` does not fix archive anomaly | R1 strategy #3 (deep investigation) is the documented fallback; no Sub-Set e2e fallback per D3 |
| R2 | Parser grammar change breaks existing AST → IR pipeline | TDD with 33-example test suite as oracle; git revert if any regression |
| R3 | fmt_ffi 8-arg migration breaks `stdlib/fmt.ry` callers | `stdlib/fmt.ry` has no callers using `__string_replace_all` directly in the current tree; verification via new `fmt_demo.ry` example + 4 unit tests in `fmt_ffi.rs` |
| R4 | R5 table refactor introduces new clippy lints | R4's snapshot diff is the gate; any new lint in the R5 commit blocks merge |
| R5 | Codegen ABI change breaks runtime symbol resolution | All 35 FFI entries are exercised by `cargo test -p ruyi_runtime --lib`; any missing entry surfaces immediately |
| R6 | LTO-disabled performance regression is unacceptable | Acceptable per D3: compiler binaries are short-lived; if a future requirement demands LTO, the fix is to either (a) split `ruyi_runtime` into core+ffi sub-crates, OR (b) move FFI tables to rlib-only |

## Migration & Rollback

**Migration**: none — v0.5.9 is a follow-up to v0.5.8 with no migration path needed.

**Rollback**: `git revert <merge commit>` reverts the entire v0.5.9 change in one operation. v0.5.8 stays the last-good release on main.

If only one sub-batch needs to be reverted (e.g., R5 causes a regression), `git revert <sub-batch commit>` is per-sub-batch atomic — each sub-batch is its own commit on `dev/v0.5.9-stdlib-cleanup`.

## Open Questions

- Q1: When R1 strategy 3 is invoked, what is the expected time-box? (default: 30 min; escalate to v0.5.10 split if exceeded)
- Q2: For R5's static table, should `&'static [BuiltinDecl]` live in a new `crates/ruyic/src/codegen/builtins_table.rs` file, or inline in `builtins.rs`? (default: new file `builtins_table.rs` for clarity)
- Q3: R3 leaves the 3-arg `__string_replace_all_legacy` in place. When should it be deleted? (default: v0.6.0, after one release cycle of deprecation)
