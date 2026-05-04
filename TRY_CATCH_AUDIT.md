# TRY_CATCH_AUDIT.md

**Task**: Audit try/catch code generation
**Date**: 2026-05-04
**Status**: READ-ONLY ANALYSIS COMPLETE

---

## 1. compile_try Implementation Analysis

**File**: `crates/ruyic/src/codegen/stmt.rs:245-378`

### Key Findings

#### A. Control Flow Structure
The `compile_try` function creates these basic blocks:
- `try_body_bb` - executes the try body
- `try_catch` - handler block (if catch clause exists)
- `try_finally` - finally block (if present)
- `try_propagate` - rethrow path when finally without catch
- `try_merge` - join point after try/catch/finally

#### B. Exception Pointer Setup (lines 266-273)
```rust
let exception_ptr = ctx.builder.build_alloca(i8_ptr, "exc_ptr");
ctx.builder.build_store(exception_ptr, i8_ptr.const_null());
let clear_fn = ctx.module.get_function("ruyi_clear_pending_exception")...;
ctx.builder.build_call(clear_fn, &[], "clear_exc");
```
Uses a manually allocated exception pointer, NOT LLVM landing pads.

#### C. CRITICAL: Uses `build_call` NOT `build_invoke`

**Line 273**: `ctx.builder.build_call(clear_fn, &[], "clear_exc")`

Inside `compile_block` (called at line 286), function calls use `build_call`:
- `expr.rs:889`: `ctx.builder.build_call(func, &arg_values, "call")`

**No `invoke` instructions are generated anywhere in the try block.**

This is a significant gap. In LLVM exception handling, `invoke` is required inside try regions because it specifies a landing pad for unwinding. Regular `call` instructions cannot route exceptions to catch handlers.

### Problem
The current implementation uses `call` for all function calls in try blocks. When an exception is thrown:
1. The exception propagates up the stack (not to the catch block)
2. The simple try_stack-based control flow cannot intercept exceptions from called functions
3. Only explicit `throw` statements (compile_throw) benefit from the try_stack mechanism

---

## 2. compile_throw Implementation Analysis

**File**: `crates/ruyic/src/codegen/stmt.rs:185-242`

### Key Findings

#### A. ruyi_throw Call (line 196)
```rust
ctx.builder.build_call(throw_fn, &[exc_ptr.into()], "throw");
```
Uses `build_call` (not `build_invoke`).

#### B. Control Flow After Throw (lines 198-206)
```rust
if let Some(try_ctx) = ctx.try_stack.last() {
    ctx.builder.build_store(try_ctx.exception_ptr, exc_ptr);
    if let Some(catch_bb) = try_ctx.catch_bb {
        ctx.builder.build_unconditional_branch(catch_bb);
    } else if let Some(finally_bb) = try_ctx.finally_bb {
        ctx.builder.build_unconditional_branch(finally_bb);
    } else {
        ctx.builder.build_unconditional_branch(try_ctx.merge_bb);
    }
}
```

**Mechanism**: `compile_throw` uses a manual try_stack to route control flow:
1. Stores exception pointer
2. Branches to catch/finally/merge

**Limitation**: This only works for explicit `throw` statements. Exceptions from called functions bypass this mechanism.

#### C. No-try Context Handling (lines 207-240)
When no try_stack entry exists, `ruyi_throw` returns a default value and the function returns. This is incomplete - a noreturn function should use `unreachable`.

---

## 3. LandingPadGenerator Interface Analysis

**File**: `crates/ruyi_runtime/src/exception/landing_pad.rs`

### Interface Definition
```rust
pub struct LandingPadGenerator<'ctx, 'm, 'b> {
    context: &'ctx Context,
    module: &'m Module<'ctx>,
    builder: &'b Builder<'ctx>,
}

impl LandingPadGenerator {
    pub fn build_landing_pad(&self, catch_type_ids: &[TypeId], has_cleanup: bool, name: &str) -> BasicValueEnum<'ctx>
    pub fn build_invoke(&self, fn_val: FunctionValue<'ctx>, args: &[BasicValueEnum<'ctx>], then_bb: BasicBlock<'ctx>, catch_bb: BasicBlock<'ctx>, name: &str) -> CallSiteValue<'ctx>
    pub fn build_resume(&self, landing_pad_val: BasicValueEnum<'ctx>)
    pub fn extract_exception_ptr(&self, landing_pad_val: BasicValueEnum<'ctx>) -> PointerValue<'ctx>
    pub fn extract_selector(&self, landing_pad_val: BasicValueEnum<'ctx>) -> IntValue<'ctx>
    pub fn build_eh_typeid_for(&self, type_id: TypeId) -> IntValue<'ctx>
    pub fn build_catch_dispatch(&self, landing_pad_val: BasicValueEnum<'ctx>, catch_handlers: &[(TypeId, BasicBlock<'ctx>)], cleanup_bb: Option<BasicBlock<'ctx>>, resume_bb: BasicBlock<'ctx>)
}
```

### Compatibility Assessment

**Current State**: NOT connected to codegen
- Located in `ruyi_runtime` crate, not accessible to `ruyic` codegen
- Exists behind `#[cfg(feature = "inkwell")]`
- Used only in `runtime.rs` tests (lines 106-259)

**Required for T7**: LandingPadGenerator needs to be accessible from `ruyic` codegen and integrated into:
1. `compile_try` - to generate `invoke` instead of `call` for try-body functions
2. Catch dispatch - to use `landingpad` instruction and selector matching
3. Resume propagation - to handle uncaught exceptions

---

## 4. T7 Modification Recommendations

### Files T7 Must Modify

| File | Changes Needed |
|------|----------------|
| `crates/ruyic/src/codegen/stmt.rs` | Integrate LandingPadGenerator into compile_try; replace build_call with build_invoke for try-body functions |
| `crates/ruyic/src/codegen/expr.rs` | Replace build_call with build_invoke when inside try context |
| `crates/ruyi_runtime/src/exception/landing_pad.rs` | May need to be made available to ruyic (move to shared crate or make public) |

### Key Changes Required

1. **compile_try** (stmt.rs:245-378):
   - Add LandingPadGenerator state to CodegenContext
   - Wrap function calls in try body with `build_invoke` instead of `build_call`
   - Generate landing pad instruction for catch handlers

2. **compile_throw** (stmt.rs:185-242):
   - Add `unreachable` after `ruyi_throw` call (noreturn function)
   - Consider whether try_stack mechanism can be replaced with landing pad

3. **expr.rs:889**:
   - When inside try context, use `invoke` instead of `call`

### Architectural Decision

The LandingPadGenerator in `ruyi_runtime` has lifetime parameters `<'ctx, 'm, 'b>` tied to inkwell types. For integration:
- Move LandingPadGenerator to a shared `ruyi_exception` crate accessible by both `ruyic` and `ruyi_runtime`
- OR make the landing pad generation logic available as a standalone helper
- OR have ruyic implement its own LandingPadGenerator using the same interface

---

## 5. Summary

| Question | Answer |
|----------|--------|
| Does compile_try use invoke? | **NO** - currently uses build_call for all function calls |
| Does compile_try use call? | **YES** - all function calls use build_call |
| How does ruyi_throw interact with control flow? | Uses try_stack to branch to catch/finally/merge after storing exception pointer |
| Is LandingPadGenerator compatible with codegen? | **NO** - not accessible from ruyic, needs architectural change |
| What T7 needs to modify | stmt.rs, expr.rs, potentially move/refactor LandingPadGenerator |

---

## Evidence

See: `.sisyphus/evidence/task-6-audit.md`