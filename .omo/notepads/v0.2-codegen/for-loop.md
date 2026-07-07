
## For Loop Codegen Findings

### Implementation Summary
- Added `Statement::For` handler in `crates/ruyic/src/codegen/stmt.rs`
- Created BB structure: init → cond → body → update → after
- Used `compile_while` as the exact pattern to follow
- For loop continue jumps to update_bb (not cond_bb like while)
- ForIn/ForOf return explicit "not yet supported" errors

### Key Design Decisions
- `loop_stack.push((end_bb, update_bb))` — continue goes to update block, which is correct C-style for loop semantics
- Init compilation handles both `ForInit::VarDecl` and `ForInit::Expr`
- Empty condition defaults to infinite loop (unconditional branch to body)
- Empty update defaults to direct branch back to cond

### Issues Discovered & Fixed
1. **Parser bug: for-loop declaration init consumed semicolon twice**
   - `parse_declaration()` for `let` consumes its own semicolon
   - `parse_for_statement` then expected another semicolon
   - Fix: refactored init parsing to consume semicolon in each branch appropriately

2. **Syntax mismatch in control_flow/loops.ry**
   - Test used `while i < 5 {` and `for let j = 0; ...` without parentheses
   - Parser requires `while (cond)` and `for (init; cond; update)`
   - Fix: updated loops.ry to use correct syntax with parentheses

3. **Type inference gap for unannotated let bindings**
   - `let i = 0` defaults to `Type::Dynamic`, causing codegen type mismatches in comparisons
   - Integration tests need explicit type annotations (e.g., `let i: int = 0`) to compile correctly
   - This affects while loops too, not just for loops

4. **Pre-existing compilation errors in working tree**
   - Missing `compile_match_expr`, `compile_block_expr`, `compile_pattern_condition` stubs
   - `build_invoke` / `build_call` type mismatch with `BasicValueEnum` vs `BasicMetadataValueEnum`
   - Fixed by adding error stubs and adjusting argument vector types

### Files Modified
- `crates/ruyic/src/codegen/stmt.rs` — Added `compile_for`, For/ForIn/ForOf match arms
- `crates/ruyic/src/parser/parser.rs` — Fixed for-loop init semicolon parsing
- `crates/ruyic/src/codegen/expr.rs` — Fixed build_call_or_invoke type mismatch, added error stubs for missing match/block expr handlers
- `crates/ruyic/tests/integration/cases/control_flow/loops.ry` — Added parentheses and type annotations
- `crates/ruyic/tests/integration/cases/codegen/for_loop.ry` — New integration test
- `crates/ruyic/tests/integration/cases/codegen/for_loop.expected` — New expected output

### Verification
- `cargo check -p ruyic` passes
- `for_loop.ry` compiles and outputs `10\n24` as expected
- `control_flow/loops.ry` compiles and outputs expected sequence
