# task-6-audit.md

**Task**: Audit try/catch code generation
**Date**: 2026-05-04

## Evidence Collected

### 1. compile_try Implementation (stmt.rs:245-378)

Key lines examined:
- 266-273: Exception pointer setup with `build_alloca` and `build_call` to clear_fn
- 286: `compile_block(ctx, body)` - executes try body (uses `build_call` for function calls)
- 289-294: Unconditional branch to finally or merge if body has no terminator

**Critical observation**: No `invoke` instruction generation found.

### 2. compile_throw Implementation (stmt.rs:185-242)

Key lines examined:
- 196: `ctx.builder.build_call(throw_fn, &[exc_ptr.into()], "throw")` - calls ruyi_throw
- 198-206: Manual control flow via try_stack

**Critical observation**: Only handles explicit `throw` statements, not exceptions from called functions.

### 3. LandingPadGenerator (landing_pad.rs)

Located at: `crates/ruyi_runtime/src/exception/landing_pad.rs`

Exposes:
- `build_landing_pad()` - generates `landingpad` instruction
- `build_invoke()` - generates `invoke` instruction
- `build_catch_dispatch()` - selector-based dispatch to handlers

**Critical observation**: In `runtime.rs`, used only in `#[cfg(feature = "inkwell")] llvm` module for tests. NOT connected to actual codegen.

### 4. Function call pattern (expr.rs:889)

```rust
let call_site = ctx.builder.build_call(func, &arg_values, "call");
```

Used for all function calls, including inside try blocks. Should be `build_invoke` when in try context.

## Grep Results

```
crates/ruyic/src/codegen/stmt.rs:
  196: ctx.builder.build_call(throw_fn, &[exc_ptr.into()], "throw");
  273: ctx.builder.build_call(clear_fn, &[], "clear_exc");
  367: .build_call(throw_fn, &[exc_val2.into()], "rethrow");

crates/ruyi_runtime/src/exception/landing_pad.rs:
  73-83: build_invoke method exists in LandingPadGenerator

No build_invoke usage found in ruyic/src/codegen/
```

## Conclusion

The try/catch implementation is INCOMPLETE for LLVM exception handling. It uses a manual (non-LLVM) approach with try_stack for explicit throws, but cannot properly handle exceptions thrown from called functions because it uses `call` instead of `invoke`.