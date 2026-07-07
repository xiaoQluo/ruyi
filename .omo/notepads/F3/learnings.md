# F3 QA Learnings

## Test Execution Findings

### Working Features
1. **Hello World**: Basic print statement works correctly
2. **Exception Handling**: try/catch/throw works at runtime
3. **Async Functions**: LLVM IR generation creates `$new` and `$poll` state machine functions
4. **Empty Program**: Compiles and runs successfully
5. **Long Strings**: Handles strings up to 300+ chars correctly
6. **Multiple Async Functions**: Compiles successfully

### Compilation Output
- LLVM warnings about unused imports in `ruyi_runtime/src/arc.rs` are benign
- `WeakRef.ptr` field is never read but is part of the design (dead code warning)
- All compilations succeed when the IR can be generated

## Patterns Observed
- Async state machine pattern uses `$new` constructor and `$poll` executor
- Object member access is NOT yet supported (codegen error for `o.x`)
- Linking step may fail if temp object files are cleaned up too early