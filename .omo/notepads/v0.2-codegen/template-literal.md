# Template Literal Codegen (T16)

## What was done

Added `Expr::TemplateLiteral` handler in `crates/ruyic/src/codegen/expr.rs` that lowers template literals to a chain of string concatenation calls.

## Key Implementation Details

- **TemplatePart enum** from `parser/ast.rs` has two variants:
  - `String(String)` - raw string segments
  - `Expr(Box<Expr>)` - interpolated expressions

- **Lowering strategy**: Each template part is compiled and concatenated left-to-right using `ruyi_string_concat`.

- **Empty template** (` `` `) returns an empty global string pointer.

- **Expression conversion**: Non-string values (int, float) are converted to string using existing runtime functions:
  - `ruyi_int_to_string` via `build_int_to_string` 
  - `ruyi_float_to_string` via `build_float_to_string`
  - String pointers pass through directly

- **Import needed**: Added `TemplatePart` to imports and `build_int_to_string`, `build_float_to_string` from builtins.

## Files Modified

- `crates/ruyic/src/codegen/expr.rs`:
  - Added `TemplatePart` to imports
  - Added `compile_template_literal` and `compile_template_part` functions
  - Added handler in `compile_expr` match arm
  - Imported `build_int_to_string` and `build_float_to_string` from builtins

- `crates/ruyic/tests/codegen.rs`:
  - Added `codegen_template_literal` test with three test cases

## Test Cases Added

1. `let name = "world"; print("Hello ${name}");` → "Hello world"
2. `let a = 1; let b = 2; print("${a} + ${b} = ${a + b}");` → "1 + 2 = 3"  
3. `let empty = ""; print("val: ${empty}");` → "val: "

## Notes

- Tagged templates NOT implemented (per requirements)
- No separate runtime function for templates - reuses existing string concat
- LLVM not available in current environment to verify end-to-end
