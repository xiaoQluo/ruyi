# BuiltinRegistry Infrastructure - Learnings

## Approach
- Created `BuiltinRegistry` with `HashMap<String, BuiltinEntry>` mapping .ry source names (`__builtin_*`) to runtime C symbols (`ruyi_*`).
- Used `BuiltinDeclareFn = for<'ctx> fn(&'ctx Context, &Module<'ctx>)` for declaration function pointers.
- Preserved `print` and `spawn` special-cases in `compile_call` before registry lookup.
- Added `is_builtin_name()` to gate registry lookups on known prefixes.

## Files Modified
- `crates/ruyic/src/codegen/builtins.rs`: Registry + declaration functions
- `crates/ruyic/src/codegen/expr.rs`: Registry lookup in `compile_call`
- `crates/ruyic/src/codegen/generator.rs`: Pre-declare registry builtins
- `crates/ruyic/src/typechecker/inference.rs`: Type env entries for registered builtins

## Verification
- `cargo check --workspace` passes with LLVM 14 prefix set.

## Notes
- Function pointer field access requires parentheses: `(entry.declare)(ctx, module)`.
- Resend sandbox domain (`onboarding@resend.dev`) can only deliver to account email; used `xiaoq.luo@foxmail.com` as fallback.
