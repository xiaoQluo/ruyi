# Spec 01: archive-anomaly — Fix `libruyi_runtime.a` packaging for binary e2e

## Overview

`v0.5.8-stdlib-core` shipped 20 new FFI entries (`__math_*` / `__time_*` / `__json_*`) across the compiler's three-layer FFI surface, but `target/release/libruyi_runtime.a` is a 215 MB archive that contains only LLVM `AArch64A*.o` / `X86*.o` / `MathExtras.cpp.o` objects — **none of the ruyi_runtime object files** (`math_ffi.o`, `time_ffi.o`, `json_ffi.o`, `gc.o`, `builtins.o`, `async_runtime.o`, `async_gc_roots.o`, `exception.o`, `alloc.o`, `arc.o`, `async_exports.o`, `gc_exports.o`) are packed into the archive. This blocks binary end-to-end validation: `make run-example EXAMPLE=math_demo` produces a binary that calls `__math_abs` etc., but at link time the linker cannot resolve these symbols (they're "unresolved" or "undefined" at link time, manifesting as silent zero-return at runtime, not a link error).

This spec fixes the archive anomaly by adjusting the release profile to disable LTO and revert to the default 16-unit codegen split.

## Requirements

### REQ-1: Release profile change
**SHALL** modify `Cargo.toml` `[profile.release]` to:
- `lto = false` (was `lto = true`)
- `codegen-units = 16` (was `codegen-units = 1`)

**Why**: The root cause hypothesis (validated empirically) is that `lto = true` + `codegen-units = 1` causes cargo to merge the ruyi_runtime object files into inkwell's LLVM static archive during the `staticlib` emission step. Disabling LTO and reverting to the default 16-unit split keeps the ruyi_runtime objects as separate `.o` files in the archive, which is the correct shape for downstream linking.

### REQ-2: Verification gate
**SHALL** validate the fix via:
```bash
cargo clean -p ruyi_runtime
cargo build --release
ar t target/release/libruyi_runtime.a | grep -E 'math_ffi|time_ffi|json_ffi' | wc -l   # ≥ 3
nm target/release/libruyi_runtime.a | grep -E '__math_abs|__time_now|__json_parse' | wc -l  # ≥ 3
```

**Why**: `ar t` lists archive members; `nm` lists defined symbols. Together they confirm the 3 new FFI files are packed and the 3 example symbols are exported.

### REQ-3: Binary e2e smoke test
**SHALL** validate the fix by running `make run-example EXAMPLE=math_demo` and observing:
- Exit code 0
- Output contains `PI ≈ 3.14159...`, `sqrt(16) = 4.000000`, `abs(-3.5) = 3.500000` (or similar expected values)
- No segfault, no "undefined symbol" runtime error

**Why**: This is the **Full e2e acceptance** for v0.5.9. The archive fix is unverified unless a real binary can call into the new FFI and produce correct output.

## Scenarios

### SCEN-1: archive verification
**WHEN** `cargo clean -p ruyi_runtime && cargo build --release` completes
**THEN** `target/release/libruyi_runtime.a` contains at least:
- `math_ffi.o` (from `crates/ruyi_runtime/src/math_ffi.rs`)
- `time_ffi.o` (from `crates/ruyi_runtime/src/time_ffi.rs`)
- `json_ffi.o` (from `crates/ruyi_runtime/src/json_ffi.rs`)

**Acceptance**:
- `ar t target/release/libruyi_runtime.a | grep -cE 'math_ffi|time_ffi|json_ffi'` ≥ 3
- `nm target/release/libruyi_runtime.a | grep -E '__math_abs|__time_now|__json_parse'` returns 3 lines

### SCEN-2: binary runtime FFI resolution
**WHEN** `make run-example EXAMPLE=math_demo` is invoked
**THEN** the resulting binary successfully calls into the runtime FFI and prints expected values.

**Acceptance**:
- Exit code 0
- Output contains numerical values matching the expected math
- No "Illegal instruction" or "Bus error" runtime failure

### SCEN-3: pre-existing examples unaffected
**WHEN** `bash examples/run_examples.sh` runs after the archive fix
**THEN** 33/33 examples pass (no regression from the LTO-disabled setting)

**Acceptance**: 33/33 exit 0

## Out of Scope

- LTO performance regression quantification (acceptable per design.md D3: ~5-10% slower at peak, ~10-15% larger binary, negligible for short-lived compiler)
- ruyi_runtime multi-crate split (R1 strategy #2 — rejected in design.md D3)
- Compilation time impact of `codegen-units=16` (expected: faster due to parallelism; no measurement required)
- Deep investigation of the LTO pipeline (R1 strategy #3 — only if SCEN-1 fails)

## Fallback

If `lto=false + codegen-units=16` does not produce a passing SCEN-1:
- **Step 1**: inspect the archive with `ar t target/release/libruyi_runtime.a | head -20` to confirm the pattern
- **Step 2**: try `lto = "thin"` (a middle-ground LTO mode) instead of `lto = false`
- **Step 3**: try `codegen-units = 1` + `lto = false` (keep codegen-units=1, only disable LTO)
- **Step 4 (R1 strategy #3)**: deep investigation of the cargo staticlib emission pipeline
- **If all fail**: per design.md D3, **no Sub-Set e2e fallback** — split v0.5.9 into a partial release (T2 + T3 + T4 + T5 as `v0.5.9-partial`) and defer T1 to `v0.5.10`
