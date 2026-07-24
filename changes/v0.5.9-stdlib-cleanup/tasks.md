# Tasks: v0.5.9-stdlib-cleanup

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `Cargo.toml` | Modify | Release profile: `lto=false` + `codegen-units=16` |
| `crates/ruyic/src/codegen/builtins_table.rs` | **Create** | `BuiltinSig` enum + `BuiltinDecl` struct + `BUILTINS` static table + dispatch helpers |
| `crates/ruyic/src/codegen/builtins.rs` | Modify | Replace 60+ hand-written `fn declare_*` with `declare_builtins` iterating the table |
| `crates/ruyic/src/typechecker/inference.rs` | Modify | `resolve_builtin_name` walks `BUILTINS` table |
| `crates/ruyic/src/parser/parser.rs` | Modify | 3 grammar productions: `dyn` return, `dyn` param, `?:` optional |
| `crates/ruyi_runtime/src/builtins.rs` | Modify | `__string_replace_all` → `__string_replace_all_legacy` (3-arg deprecation) |
| `crates/ruyi_runtime/src/fmt_ffi.rs` | Modify | `ruyi_string_replace_all` → `__string_replace_all` (8-arg canonical) |
| `examples/fmt_demo.ry` | **Create** | R3 verification example |
| `changes/v0.5.9-stdlib-cleanup/` | **Create** | 9 件 spec-superflow 法度 |

---

## Sub-batch 1 (R1): runtime archive anomaly

### T1.1 — Modify release profile
- [x] `Cargo.toml` `[profile.release]`: `lto = true` → `lto = false`
- [x] `Cargo.toml` `[profile.release]`: `codegen-units = 1` → `codegen-units = 16`

**Validation:**
```bash
cargo clean -p ruyi_runtime
cargo build --release
ar t target/release/libruyi_runtime.a | grep -E 'math_ffi|time_ffi|json_ffi' | wc -l   # ≥ 3
nm target/release/libruyi_runtime.a | grep -E '__math_abs|__time_now|__json_parse' | wc -l  # ≥ 3
```

### T1.2 — Verify binary e2e
- [x] `make run-example EXAMPLE=math_demo` produces a running binary
- [x] Output contains: `PI = 3.14159...`, `sqrt(16) = 4.000000`, `abs(-3.5) = 3.500000`

**Fallback (T1.1 fails):** immediately escalate to R1 strategy #3 (deep investigation). Per D3 in design.md, **no Sub-Set e2e fallback** — escalate or split v0.5.9 into partial release + R1 deferred to v0.5.10.

**Estimated time:** 30 min for T1.1, 30 min for verification. **Total: 1 hour.**

---

## Sub-batch 2 (R5): table-driven codegen

### T2.1 — Create `BUILTINS` table
- [x] Create `crates/ruyic/src/codegen/builtins_table.rs`
- [x] Define `BuiltinSig` enum: `Void`, `Int` (i64), `Float` (f64), `String` (*mut i8), `Ptr` (*mut i8 opaque)
- [x] Define `BuiltinDecl` struct: `name: &'static str`, `ret: BuiltinSig`, `params: &'static [BuiltinSig]`
- [x] Populate `pub static BUILTINS: &[BuiltinDecl]` with all 35 entries:
  - 6 `__builtin_array_*`
  - 7 `__builtin_map_*`
  - 4 `__builtin_set_*`
  - 18 `__string_*`
  - 14 `__math_*`
  - 4 `__time_*`
  - 2 `__json_*`

**TDD pattern:** Write the table first; verify each entry via a smoke-test that calls `declare_builtins()` and checks the function is declared.

### T2.2 — Refactor `codegen/builtins.rs`
- [x] Replace 60+ `fn declare_*<'ctx>(context, module)` with one iteration over `BUILTINS`:
  ```rust
  pub fn declare_builtins<'ctx>(context: &'ctx Context, module: &Module<'ctx>, gc_mode: GcMode) {
      // Special cases first (printf, alloc/gc_alloc, gc_collect, etc.)
      declare_printf(context, module);
      declare_alloc(context, module, gc_mode);
      declare_gc_collect(context, module);
      // ... etc ...

      // Bulk: iterate BUILTINS
      for d in BUILTINS {
          let fn_type = sig_to_fn_type(context, d.ret, d.params);
          module.add_function(d.name, fn_type, None);
      }
  }
  ```
- [x] `sig_to_fn_type` helper: maps `BuiltinSig` to inkwell `BasicTypeEnum` + builds `fn_type`
- [x] Verify all 60+ hand-written functions are removed (or at most 3-4 special cases like `__pow` / `__fmt_*`)

### T2.3 — Refactor `typechecker/inference.rs::resolve_builtin_name`
- [x] Walk `BUILTINS` to find matching name, map `BuiltinSig` to Ruyi `Type`:
  - `Void` → `Type::Void`
  - `Int` → `Type::Int`
  - `Float` → `Type::Float`
  - `String` → `Type::String`
  - `Ptr` → `Type::Dynamic`
- [x] Keep special cases (`RangeError`, `ArrayIterator` as `Type::Named`)

### T2.4 — Verify
- [x] `cargo check --workspace` exit 0
- [x] `cargo build --release` exit 0
- [x] `ruyic --check stdlib/collections.ry stdlib/string.ry stdlib/io.ry` all pass
- [x] `cargo test --workspace` no regression

**Estimated time:** 4 hours (table population is the most labor-intensive part; 35 entries × 3 lines ≈ 100 lines of static data).

---

## Sub-batch 3 (R2): parser fixes

### T3.1 — `dyn` as return type
- [x] In `parser/parser.rs` `parse_return_type` (or equivalent): when seeing `Token::Dyn` followed by `{` or `=>`, treat as `Type::Dynamic`
- [x] Test: `fn f(): dyn { return 1; }` parses
- [x] Verify: `ruyic --check` of any program with `dyn` return type succeeds

### T3.2 — `dyn` as parameter type
- [x] In `parser/parser.rs` `parse_param_type`: when seeing `Token::Dyn` standalone (not `dyn Trait`), treat as `Type::Dynamic`
- [x] Test: `fn f(x: dyn): dyn { return x; }` parses

### T3.3 — `?:` optional parameter syntax
- [x] In `parse_param_list`: when seeing `<type>?` followed by `=` (with default value), mark parameter as optional
- [x] Propagate optional flag through typechecker (`Type::Optional` or similar)
- [x] Codegen: generate `load global optional` + null check at use site
- [x] Test: `fn f(s: int? = 0): int { return s; }` parses and type-checks

### T3.4 — Verify
- [x] `ruyic --check stdlib/json.ry` → "Type checking passed."  ← **Full e2e 关键**
- [x] `ruyic --check stdlib/random.ry` → "Type checking passed."
- [x] `bash examples/run_examples.sh` no regression (33/33)

**Estimated time:** 3 hours (parser changes are subtle; TDD with 33-example suite as oracle).

---

## Sub-batch 4 (R3): fmt_ffi 8-arg migration

### T4.1 — Rename legacy 3-arg
- [x] `crates/ruyi_runtime/src/builtins.rs:773`: `__string_replace_all` → `__string_replace_all_legacy`
- [x] Add deprecation comment: `// DEPRECATED since v0.5.9: use the 8-arg __string_replace_all from fmt_ffi.rs instead. Will be removed in v0.6.0.`

### T4.2 — Rename fmt_ffi 8-arg to canonical name
- [x] `crates/ruyi_runtime/src/fmt_ffi.rs:53`: `ruyi_string_replace_all` → `__string_replace_all`
- [x] Update all internal callers in `fmt_ffi.rs` (test functions)
- [x] Update `crates/ruyi_runtime/src/lib.rs:54` if `pub use fmt_ffi::ruyi_string_replace_all;` exists

### T4.3 — Update codegen for 8-arg ABI
- [x] In `BUILTINS` table (T2.1), `__string_replace_all` entry uses 8-arg signature
- [x] Verify: `cargo build --release` succeeds; the LLVM `declare` for `__string_replace_all` matches the 8-arg runtime ABI

### T4.4 — Create `examples/fmt_demo.ry`
- [x] Example: `let s = "hello, world"; let t = s.replace("world", "Rust"); print(t);` style demonstration
- [x] Verify: `make run-example EXAMPLE=fmt_demo` runs and prints expected output

### T4.5 — Verify
- [x] `cargo test -p ruyi_runtime --lib fmt_ffi::` → 4 tests pass
- [x] `make run-example EXAMPLE=fmt_demo` runs to completion

**Estimated time:** 2 hours.

---

## Sub-batch 5 (R4): zero-new-clippy verification

### T5.1 — Snapshot diff
- [x] On `v0.5.8` tag: `cargo clippy --workspace 2>&1 | sort > /tmp/v058_clippy.txt`
- [x] On `dev/v0.5.9-stdlib-cleanup` (current): `cargo clippy --workspace 2>&1 | sort > /tmp/v059_clippy.txt`
- [x] `diff /tmp/v058_clippy.txt /tmp/v059_clippy.txt | tee /tmp/clippy_diff.txt`
- [x] Acceptance: `/tmp/clippy_diff.txt` is empty (or contains only known false-positives from reformatting)

### T5.2 — Fix any new lints
- [x] For each new lint in `/tmp/clippy_diff.txt`, fix the source file or add `#[allow(...)]` with justification
- [x] Re-run T5.1 until diff is empty

### T5.3 — Final Full e2e check
- [x] All 33 examples pass (`bash examples/run_examples.sh`)
- [x] All 9 stdlib `.ry` files pass `--check`
- [x] `make run-example EXAMPLE=math_demo` runs
- [x] `make run-example EXAMPLE=fmt_demo` runs
- [x] `cargo test --workspace` ≥ 110 tests pass
- [x] `cargo clippy --workspace` zero new lints

**Estimated time:** 1 hour (verification + minor fixes; heavy refactor already in R5).

---

## Overall Verification

```bash
# All sub-batches complete
cargo check --workspace                          → exit 0
cargo build --release                            → exit 0
cargo test --workspace                           → ≥ 110 tests pass
cargo clippy --workspace                         → no new lints vs v0.5.8

# Stdlib e2e
for f in core string io error option result process path collections math time json random; do
  ruyic --check stdlib/$f.ry
done
# All → "Type checking passed."

# Example e2e
make run-example EXAMPLE=math_demo               → runs, prints expected values
make run-example EXAMPLE=fmt_demo                → runs, prints expected values
bash examples/run_examples.sh                     → 33/33 PASS

# Archive anomaly fix
ar t target/release/libruyi_runtime.a | grep -cE 'math_ffi|time_ffi|json_ffi'   # ≥ 3
nm target/release/libruyi_runtime.a | grep -cE '__math_abs|__time_now|__json_parse'  # ≥ 3
```

---

## Sub-batch Status

| ID | Status | Notes |
|----|--------|-------|
| T1 (R1) | ✅ complete | commit e511236; Cargo.toml lto=false + codegen-units=16 |
| T2 (R5) | ✅ complete | commit 69bdba9; BUILTINS table 55-builtin-sig dispatch |
| T3 (R2) | ✅ complete | commit 8c37891; parser dyn-as-type + optional parameter |
| T4 (R3) | ✅ complete | commit 4f4d697; __string_replace_all 8-arg unification |
| T5 (R4) | ✅ complete | commit 6b622e7; clippy snapshot diff + e2e gate zero-regression |
