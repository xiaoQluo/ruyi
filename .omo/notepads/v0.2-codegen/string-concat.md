# String Concatenation Codegen (T15)

## Changes Made

### Runtime (`crates/ruyi_runtime/`)
- Added `ruyi_int_to_string(i64) -> *mut i8` to `builtins.rs`
- Added `ruyi_float_to_string(f64) -> *mut i8` to `builtins.rs`
- Exported both from `lib.rs`
- Added unit tests for both functions (positive, negative int, float)

### Compiler Codegen (`crates/ruyic/src/codegen/`)
- Added declarations for `ruyi_int_to_string` and `ruyi_float_to_string` in `builtins.rs`
- Added `build_int_to_string` and `build_float_to_string` helpers in `builtins.rs`
- Refactored `compile_add` in `expr.rs` from **value-based** dispatch to **type-based** dispatch:
  - `(Type::String, Type::String)` → `ruyi_string_concat`
  - `(Type::String, Type::Int)` → convert int to string, then concat
  - `(Type::Int, Type::String)` → convert int to string, then concat
  - `(Type::String, Type::Float)` → convert float to string, then concat
  - `(Type::Float, Type::String)` → convert float to string, then concat
  - Fallback preserves existing numeric addition logic exactly

### Tests
- Created `crates/ruyic/tests/integration/cases/codegen/string_concat.ry`
- Created `crates/ruyic/tests/integration/cases/codegen/string_concat.expected`
- Extended ignored `codegen_string_concat` test in `tests/codegen.rs` to cover mixed cases

## Design Decisions

1. **Type-based dispatch over value-based dispatch**: The old code matched on `BasicValueEnum::PointerValue` for any pointer + pointer, which could incorrectly concatenate arrays or objects. Using `ExprResult.ty` is safer and more explicit.

2. **Runtime helper functions over inline snprintf**: Adding `ruyi_int_to_string` and `ruyi_float_to_string` to the runtime keeps the codegen simple (just a function call) and reuses Rust's `format!()` which handles edge cases (negative numbers, float formatting) correctly.

3. **Consistent with existing patterns**: The new runtime functions and build helpers follow the exact same pattern as `ruyi_string_concat` / `build_string_concat`.

## Verification

- `cargo check -p ruyi_runtime --no-default-features` ✅
- `cargo test -p ruyi_runtime --no-default-features` ✅ (83 tests passed)
- `cargo check -p ruyic` ❌ (blocked by missing LLVM in this environment)

## Blockers / Limitations

- Full `ruyic` check and integration test execution require LLVM 14-18, which is not installed in this environment.
- The float formatting behavior relies on Rust's `format!("{}", n)` which produces compact output (`3.14`) rather than C's `%f` (`3.140000`). This is actually desirable for string concatenation but means `print(3.14)` and `print("" + 3.14)` may produce slightly different formatting.
