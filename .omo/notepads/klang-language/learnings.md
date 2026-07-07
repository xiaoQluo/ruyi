# Klang Language - Task 1 Learnings

## Project Scaffolding Patterns

### Workspace Structure
- Root `Cargo.toml` defines workspace with `resolver = "2"`
- Two crates: `klangc` (binary) and `klang_runtime` (library)
- Crates are in `crates/` subdirectory

### LLVM Version Handling
- inkwell requires LLVM 14-18 feature flags
- LLVM must be installed separately (brew install llvm@14)
- Set `LLVM_SYS_140_PREFIX=/usr/local/opt/llvm@14` for build
- inkwell version 0.2.0 works with LLVM 14

### Module Organization
- klangc: `src/lexer/`, `src/parser/`, `src/typechecker/`, `src/codegen/`, `src/gc/`, `src/runtime/`
- klang_runtime: `gc.rs`, `exception.rs`, `alloc.rs` (flat structure, not nested modules)
## Task 9: Exception Runtime LLVM Implementation

### Successful Approaches
- Using `exception.rs` as module root with `exception/types.rs`, `exception/landing_pad.rs`, `exception/runtime.rs` submodules works well in Rust 2021 edition.
- Inkwell 0.2 API differences from newer versions:
  - `get_or_insert_function` does not exist; use `get_function` + `add_function` fallback
  - `build_call` expects `BasicMetadataValueEnum`, so use `.into()` on `BasicValueEnum`
  - `PointerValue`/`IntValue` don't have `is_pointer_value()`/`is_int_value()` methods
  - Multi-lifetime structs (`<'ctx, 'm, 'b>`) are needed to avoid borrow checker issues when storing references to `Context`, `Module`, and `Builder`
- Landing pad generation uses `__gxx_personality_v0` for Itanium ABI compatibility
- `llvm.eh.typeid.for` intrinsic returns the selector value for catch dispatch

### Pre-existing Issues Fixed
- `gc.rs` referenced missing modules `generational`, `old`, `young`
- `gc/barrier.rs` and `gc/roots.rs` used `klang_dealloc` without importing it
- Created `gc/generational.rs` stub to enable compilation

