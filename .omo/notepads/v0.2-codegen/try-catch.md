# Try/Catch/Finally Codegen Implementation Notes

## Date: 2026-05-03

## Summary

Implemented try/catch/finally + throw codegen for the Ruyi compiler using LLVM invoke/landingpad with `__gxx_personality_v0`. The integration test `errors/try_catch.ry` passes.

## Changes Made

### 1. Compiler (`crates/ruyic/src/codegen/`)

#### `generator.rs`
- Added `exception_stack: Vec<BasicBlock>` to `CodegenContext`
- Added linking with `-lc++` and static runtime library (`libruyi_runtime.a`)

#### `stmt.rs`
- Fixed pre-existing syntax error: `fn compile_return'<ctx>` → `fn compile_return<'ctx>`
- Implemented `compile_try()`: generates try_body, catch_lpad (landingpad with `catch i8* null`), catch_body, resume, merge blocks
- Implemented `compile_throw()`: uses `invoke` (not `call`) when inside a try region so the unwinder can find the landingpad

#### `expr.rs`
- Implemented `compile_error_ctor()` for `Error("msg")` expressions
- Added `Error` class fields to `CodegenContext::class_fields`
- Added missing functions: `compile_bigint_literal`, `get_array_elem_type`, `unbox_value`
- Added `build_call_or_invoke()` helper to use `invoke` inside try regions

### 2. Typechecker (`crates/ruyic/src/typechecker/inference.rs`)

- Added `Error` built-in type declaration to the inference environment

### 3. Runtime (`crates/ruyi_runtime/src/builtins.rs`)

- Added `ruyi_throw()` wrapper using C++ ABI (`__cxa_allocate_exception` + `__cxa_throw`) instead of Rust panic
- Added `ruyi_begin_catch()` wrapper that adds **32-byte offset** to skip `_Unwind_Exception` header on macOS x86_64
- Added `ruyi_end_catch()` wrapper calling `__cxa_end_catch()`
- Added `ruyi_finally()` wrapper (existing)

### 4. Integration Test

- Test file: `crates/ruyic/tests/integration/cases/errors/try_catch.ry`
- Expected output: `crates/ruyic/tests/integration/cases/errors/try_catch.expected`
- **Status: PASS**

## Key Technical Discovery

### The 32-Byte Offset Problem

When using `__cxa_throw` with `__gxx_personality_v0` and `catch i8* null`, the LLVM landingpad returns the `_Unwind_Exception` header pointer, NOT the user exception object pointer.

On macOS x86_64:
- `sizeof(_Unwind_Exception) == 32`
- The user object (our `ExceptionObject`) starts at `header_ptr + 32`

Without this offset, accessing `e.message` reads garbage from the unwind header fields.

### Why `invoke` is Required for `throw`

The `throw` statement must use LLVM `invoke` (not `call`) when inside a try region. This ensures the unwinder can associate the throw with the correct landingpad. Using `call` + `unreachable` causes a segfault because the unwinder cannot find the catch block.

### Why `__cxa_throw` Instead of Rust Panic

Rust `panic!()` does not work when called from a C-style `main()` function (compiled by our compiler). It produces "failed to initiate panic, error 5". Using C++ `__cxa_throw` integrates correctly with LLVM landingpads.

## Verification

```bash
# Check compiler compiles
cargo check -p ruyic

# Run integration test
KLANG_BIN=$(pwd)/target/debug/ruyic cargo test -p ruyic --test integration
# Result: [PASS] errors/try_catch
```

## Remaining Work

1. **Portability**: The 32-byte offset is platform-specific. On Linux, `_Unwind_Exception` may have a different size. Consider detecting this at runtime or compile time.
2. **Proper exception cleanup**: We bypass `__cxa_begin_catch` (which expects the unwind header). `ruyi_end_catch` still calls `__cxa_end_catch` which may have mismatched state. Need to verify memory doesn't leak and nested catches work.
3. **Nested try/catch**: Not yet tested.
4. **Finally blocks**: Codegen structure exists but not fully verified.
5. **Other integration tests**: Many tests fail with "Compilation failed" — this may be unrelated to exception handling.
