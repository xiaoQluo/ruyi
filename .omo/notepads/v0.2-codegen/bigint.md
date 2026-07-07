# BigInt Literal Codegen (T11)

## Task
Add `Expr::BigIntLiteral` handler in `crates/ruyic/src/codegen/expr.rs`

## Implementation
1. Import `build_bigint_from_str` from `super::builtins`
2. Add `Expr::BigIntLiteral(n) => compile_bigint_literal(ctx, n)` in `compile_expr`
3. New function `compile_bigint_literal`:
   - Embeds numeric string as global constant via `build_global_string_ptr`
   - Calls runtime `ruyi_bigint_from_str` to construct bigint
   - Returns `*i8` pointer (Type::BigInt)

## Files Modified
- `crates/ruyic/src/codegen/expr.rs` - Added BigIntLiteral handler
- `crates/ruyic/src/codegen/stmt.rs` - Pre-existing changes (Break/Continue fix)
- `crates/ruyic/tests/integration/cases/codegen/bigint.ry` - Integration test
- `crates/ruyic/tests/integration/cases/codegen/bigint.expected` - Expected output

## Notes
- LLVM 14 not available on this system, full build fails at llvm-sys
- `cargo check -p ruyic` passes (0 errors, 6 warnings)
- Break/Continue issue in stmt.rs was a pre-existing bug that needed fixing (Option<&String> vs Option<String>)

## Status: COMPLETE (verified via cargo check)