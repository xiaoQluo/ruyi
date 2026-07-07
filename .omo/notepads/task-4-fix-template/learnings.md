# Task 4 Fix: TemplateLiteral Codegen

## Summary
Implemented missing `Expr::TemplateLiteral` codegen handler in `crates/ruyic/src/codegen/expr.rs`.

## Changes Made
1. Added `TemplatePart` to imports from `crate::parser::ast`
2. Added match arm in `compile_expr`: `Expr::TemplateLiteral(parts) => compile_template_literal(ctx, parts)`
3. Implemented `compile_template_literal` function (lines 142-207)

## Implementation Details
- Empty template → returns empty string constant
- Single string part → returns that string directly (no concat overhead)
- Multiple parts → chains using `ruyi_str_concat` runtime function
- Expression parts use `value_to_i8_ptr` helper (already exists) to convert values to i8*

## Verification
- Cannot run `cargo check -p ruyic` due to LLVM 14 not being installed on this machine
- `cargo check -p ruyi_runtime --no-default-features` passes (runtime-only, no LLVM needed)
- Code structure verified by reading the modified file

## Note
The `ruyi_str_concat` function was already declared in `builtins.rs` (line 26, 100-104) and used by `compile_add` (string + string case). This implementation follows the same pattern.