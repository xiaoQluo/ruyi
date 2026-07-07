# Decisions

## inkwell LLVM Feature Selection
- Decision: Use `llvm14-0` feature flag
- Rationale: LLVM 14 is stable and widely available via Homebrew
- Alternative considered: Higher LLVM versions (15-18) but may not be as readily available

## Crate Separation: klangc vs klang_runtime
- klangc: Contains lexer, parser, typechecker, codegen - compiler logic
- klang_runtime: Contains GC allocator, exception types, low-level runtime
- Rationale: Clean separation allows runtime to be linked as standalone library

## Module Placeholders
- Created stub modules in all subdirectories to allow compilation
- Real implementation will come in subsequent tasks (Task 4 for lexer, etc.)