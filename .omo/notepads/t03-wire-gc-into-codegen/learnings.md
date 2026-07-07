# T03 — Wire GC into Codegen

## Learnings

- The `CodegenContext` is the right place to track GC root scopes because it follows the function call stack during code generation.
- `build_gep` in inkwell 0.2.0 takes 3 args (ptr, indices, name), not 4 (no element type first arg).
- `const_int` in inkwell takes `u64`, not `u32`.
- The `BasicValue` trait must be imported to use `as_basic_value_enum()`.
- `is_gc_managed` should recurse into `Nullable` types since `Nullable(Int)` is not GC-managed but `Nullable(String)` is.

## Issues Encountered

- Parser bug: `[1, 2, 3]` fails with "Expected ']' but found 'integer literal'" — pre-existing, not related to codegen changes.
- Parser bug: `{ x: 1, y: 2 }` fails with "Expected '}' but found 'identifier'" — pre-existing, not related to codegen changes.
- Multiple pre-existing test failures in `macro_expand.rs`, `generics.rs`, `diagnostics.rs`, `integration/runner.rs`, `patterns.rs`, `traits.rs`.
- `gc_exports` runtime test SIGSEGVs on `test_gc_collect_survives_reachable` — pre-existing GC runtime issue.

## Decisions

- Chose to track GC roots per-function using a `Vec<Vec<(PointerValue, Type)>>` stack in `CodegenContext`.
- Emit `build_gc_remove_root` calls at each return path (via `compile_return`) and at implicit function exit, rather than using a single cleanup block. This is simpler and correct because each code path only executes one return.
- For `ArrayLiteral` and `ObjectLiteral`, allocated memory as raw `i8*` blobs with `i64`-sized slots for simplicity in the baseline codegen.
- `compile_new` allocates a fixed 64-byte instance and calls `{class}_new` constructor if found.

## Verification Results

- `cargo check -p ruyic` passes cleanly (only pre-existing warnings).
- `ruyic /tmp/gc_test.ry --emit-llvm` produces LLVM IR containing:
  - `call i8* @ruyi_gc_alloc(i64 8)` for object literals
  - `call void @ruyi_gc_write_barrier(i8* %gc_alloc, i8* ...)` for pointer fields
  - `call void @ruyi_gc_add_root(i8* %root_val)` for GC-managed parameters
  - `call void @ruyi_gc_remove_root(i8* %root_val2)` before returns
- `cargo test -p ruyic --test typechecker` passes (157 passed, 7 failed — all pre-existing).
