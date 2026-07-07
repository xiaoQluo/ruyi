# Break/Continue Codegen Implementation

## Date: 2026-05-02

## Changes Made

### `crates/ruyic/src/codegen/stmt.rs`

Added handlers for `Statement::Break` and `Statement::Continue` in `compile_stmt`:

```rust
Statement::Break(label) => compile_break(ctx, label.clone()),
Statement::Continue(label) => compile_continue(ctx, label.clone()),
```

Added `compile_break` function (lines 150-159):
```rust
fn compile_break<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    _label: Option<String>,
) -> Result<(), String> {
    let end_bb = ctx.loop_stack.last()
        .ok_or("break outside of loop")?
        .0;
    ctx.builder.build_unconditional_branch(end_bb);
    Ok(())
}
```

Added `compile_continue` function (lines 161-170):
```rust
fn compile_continue<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    _label: Option<String>,
) -> Result<(), String> {
    let cond_bb = ctx.loop_stack.last()
        .ok_or("continue outside of loop")?
        .1;
    ctx.builder.build_unconditional_branch(cond_bb);
    Ok(())
}
```

### Integration Tests Created

- `control_flow/break_while.ry` + `.expected` - Tests break in while loop
- `control_flow/continue_while.ry` + `.expected` - Tests continue in while loop
- `control_flow/break_for.ry` + `.expected` - Tests break in for loop

## Implementation Notes

- `loop_stack` is `Vec<(end_bb, cond_bb)>` - tuple order is (end, condition)
- Break uses `.0` (end_bb) to jump past the loop
- Continue uses `.1` (cond_bb) to jump to loop condition
- Last entry in stack = innermost loop (handles nesting correctly)
- Labels are accepted but ignored (per MUST NOT DO - no labeled breaks)

## Verification

- `cargo check -p ruyic` passes
- Full build requires LLVM 14-18 (not available in this environment)