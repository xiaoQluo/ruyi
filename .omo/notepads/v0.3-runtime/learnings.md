- `std::mem::size_of_val` works on `&dyn Trait` and returns the concrete type's size, enabling conservative memory scanning of boxed futures.
- Casting a fat pointer `*const dyn Trait` to `*const u8` yields the data pointer (the vtable is discarded), which is a safe/stable way to obtain the payload address of a trait object.
- `MarkSweepCollector::is_valid_payload` was added to safely validate candidate pointers before calling `add_root`, preventing crashes from spurious non-GC pointers found during conservative scanning.
- The `GLOBAL_SCHEDULER` static was moved from `async_exports.rs` to `async_runtime.rs` so that `register_async_roots` (which lives in `async_runtime.rs`) can access the same scheduler instance used by the C exports.

## Try/Catch/Throw Codegen Implementation

### Date: 2026-05-03

### Implementation Approach
The codegen uses a **pending-exception model** rather than LLVM invoke/landingpad:
- `ruyi_throw(msg)` stores the exception pointer in a thread-local `PENDING_EXCEPTION` and returns normally
- `ruyi_get_pending_exception()` retrieves it
- `ruyi_clear_pending_exception()` clears it
- After every expression statement, `build_exception_check` checks for pending exceptions and branches to catch/finally if present
- `compile_throw` calls `ruyi_throw`, stores the exception in `try_ctx.exception_ptr`, and branches to catch/finally/merge
- `compile_try` sets up exception pointer alloca, clears pending exception, pushes try context, compiles body, then generates catch/finally/merge blocks

### Key Bug Fixed
Nested try/catch was propagating handled exceptions to outer catches because `build_exception_check` inside the inner catch block would see the still-pending exception. Fixed by adding `build_ruyi_clear_pending_exception()` at the start of each catch block.

### Driver Fix
`driver.rs` was rebuilding `ruyi_runtime` with default features (inkwell), causing link failures due to massive LLVM C++ dependencies. Fixed by passing `--no-default-features` to the runtime build command.

### Files Modified
- `crates/ruyic/src/codegen/stmt.rs` - Added `compile_try`, `compile_throw`, `build_exception_check`
- `crates/ruyic/src/codegen/generator.rs` - Added `TryContext` and `try_stack` to `CodegenContext`
- `crates/ruyic/src/codegen/builtins.rs` - Added `ruyi_get_pending_exception`, `ruyi_clear_pending_exception`, `ruyi_str_concat` declarations
- `crates/ruyi_runtime/src/c_exports.rs` - Added thread-local pending exception model
- `crates/ruyic/src/driver.rs` - Added `--no-default-features` to runtime build

### Verification Results
- `try { throw "error"; } catch (e) { print(e); }` → outputs `error`, exit 0 ✅
- Nested try/catch → inner catch handles, outer not triggered ✅
- Try/catch/finally → catch runs, then finally ✅
