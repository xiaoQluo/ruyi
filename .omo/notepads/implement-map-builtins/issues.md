# Issues: Implement map builtins

## Blockers encountered
1. **Missing LLVM in environment**: `cargo check --workspace` fails because `llvm-sys` cannot find LLVM 14. This is a pre-existing infrastructure issue, not caused by map changes.
2. **Pre-existing broken tests in builtins.rs**: Tests reference `ruyi_array_create` and `ruyi_array_slice` which do not exist in the codebase. This prevents `cargo test -p ruyi_runtime` from compiling the test suite.

## Resolution
- Verified `cargo check -p ruyi_runtime --no-default-features` passes cleanly.
- Map module itself has no compilation errors or warnings.
- Codegen changes are purely additive and follow existing patterns exactly.
