# Spec 04: fmt-ffi-8arg — Migrate `__string_replace_all` to 8-arg bounded-buffer ABI

## Overview

`crates/ruyi_runtime/src/builtins.rs:773` defines a 3-arg `__string_replace_all(input: *const i8, pattern: *const i8, replacement: *const i8) -> *mut i8` (C-string version). `crates/ruyi_runtime/src/fmt_ffi.rs:53` defines a different, 8-arg `ruyi_string_replace_all(s: *const u8, s_len: usize, from, from_len, to, to_len, out: *mut u8, out_cap: usize) -> usize` (bounded-buffer version).

In v0.5.8, an attempt to rename `ruyi_string_replace_all` → `__string_replace_all` triggered a "symbol `__string_replace_all` is already defined" error because the 3-arg and 8-arg versions share the symbol name. The rename was reverted; the 8-arg version retained the `ruyi_` prefix.

This spec migrates codegen to the 8-arg ABI (the newer, more capable design), deprecates the 3-arg legacy (with a `_legacy` suffix to avoid the collision), and renames `fmt_ffi.rs`'s `ruyi_string_replace_all` to `__string_replace_all` (A-path naming convention).

## Requirements

### REQ-1: Rename legacy 3-arg
**SHALL** modify `crates/ruyi_runtime/src/builtins.rs:773`:
- Function name: `__string_replace_all` → `__string_replace_all_legacy`
- Add deprecation doc comment: `// DEPRECATED since v0.5.9: use the 8-arg __string_replace_all from fmt_ffi.rs. Will be removed in v0.6.0.`

### REQ-2: Promote fmt_ffi 8-arg to canonical name
**SHALL** modify `crates/ruyi_runtime/src/fmt_ffi.rs`:
- Function name: `ruyi_string_replace_all` → `__string_replace_all`
- All internal callers (test functions in the same file) updated
- The signature is unchanged: `(s: *const u8, s_len: usize, from: *const u8, from_len: usize, to: *const u8, to_len: usize, out: *mut u8, out_cap: usize) -> usize`

### REQ-3: Update codegen for 8-arg ABI
**SHALL** modify `crates/ruyic/src/codegen/builtins_table.rs` (created in T2) entry for `__string_replace_all`:
- Old: 3-arg signature `[String, String, String] -> String`
- New: 8-arg signature `[String, Int, String, Int, String, Int, String, Int] -> String`

(Where `String` maps to `*mut i8` and `Int` maps to `i64`.)

### REQ-4: Verify `crates/ruyi_runtime/src/lib.rs:54` `pub use`
**SHALL** ensure `lib.rs:54` has the correct re-export after the rename:
- Was: `pub use fmt_ffi::ruyi_string_replace_all;`
- Now: `pub use fmt_ffi::__string_replace_all;` (or the function is `pub` already and the re-export is unnecessary; check)

### REQ-5: Create `examples/fmt_demo.ry`
**SHALL** create `examples/fmt_demo.ry` demonstrating `__string_replace_all`:
```ruyi
import { ... } from "fmt";

fn main() {
    let s = "hello, world";
    let t = s.replace("world", "Rust");
    print(t);  // → "hello, Rust"
}
```

## Scenarios

### SCEN-1: 8-arg ABI in libruyi_runtime
**WHEN** `nm --defined-only target/release/libruyi_runtime.a | grep __string_replace_all`
**THEN** exactly one symbol `__string_replace_all` is defined (the 8-arg version); `__string_replace_all_legacy` is also defined (3-arg) but separately

**Acceptance**: 2 distinct symbols, no `ruyi_string_replace_all`

### SCEN-2: codegen emits 8-arg call
**WHEN** a Ruyi program calls a function that internally uses `__string_replace_all`
**THEN** the emitted LLVM IR calls `__string_replace_all` with 8 arguments (4 `i8*` + 4 `i64`)

**Acceptance**:
```bash
./target/release/ruyic --emit-llvm examples/fmt_demo.ry | grep 'call.*__string_replace_all'
# → 8-arg call signature
```

### SCEN-3: `make run-example EXAMPLE=fmt_demo` runs
**WHEN** the demo is compiled and run
**THEN** exit 0, output "hello, Rust" (or similar)

**Acceptance**: exit 0, expected output

### SCEN-4: 4 `fmt_ffi` unit tests pass
**WHEN** `cargo test -p ruyi_runtime --no-default-features --lib fmt_ffi::`
**THEN** all 4 tests pass

**Acceptance**: `test result: ok. 4 passed; 0 failed;`

## Out of Scope

- Deleting `__string_replace_all_legacy` (deferred to v0.6.0; one release cycle of deprecation)
- Migrating the runtime to use only the 8-arg version internally (out of scope; the legacy is a runtime-level C symbol, the stdlib callsites are independent)
- Performance optimization of the 8-arg implementation (the current bounded-buffer algorithm is O(n*m) worst case; v0.6+ if needed)
- Adding new methods to `stdlib/fmt.ry` (separate change)

## Risks

| ID | Risk | Mitigation |
|----|------|------------|
| R3-1 | `stdlib/fmt.ry` has direct callers of `__string_replace_all` that break | Audit `stdlib/fmt.ry` for `__string_replace_all` usage; if found, update to 8-arg call |
| R3-2 | 8-arg signature change is incompatible with external FFI users | The 3-arg legacy is kept (`_legacy` suffix); external code can migrate at their own pace |
| R3-3 | `__string_replace_all_legacy` and `__string_replace_all` get accidentally swapped in codegen | The `BUILTINS` table makes this a 1-line change; clear naming in tests + review |

## Rollback

If `make run-example EXAMPLE=fmt_demo` fails after T4:
1. `git revert <T4 commit>`
2. R3 is reverted to v0.5.8 state (3-arg is canonical, 8-arg is orphan)
3. T4 is deferred to v0.5.10
4. v0.5.9 ships with T1 + T2 + T3 + T5 only (per design.md D3: no Sub-Set e2e fallback means T4 failure doesn't block v0.5.9, only defers R3)
