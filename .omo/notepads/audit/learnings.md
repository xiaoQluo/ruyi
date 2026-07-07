## 2026-05-04: F1 Test Execution Audit

### Environment
- macOS (x86_64), LLVM 14 installed at `/usr/local/opt/llvm@14`
- `LLVM_SYS_140_PREFIX` required for building ruyic (llvm-sys link)
- `llvm-config` not on PATH by default

### Runtime Tests Summary
`cargo test -p ruyi_runtime --no-default-features` (no LLVM needed):
- Lib unit tests: 57/57 passed
- async_gc_roots: 1/1 passed
- async_runtime: 8/8 passed
- builtins: 5/5 passed
- exception: 8/8 passed
- exception_runtime: 19/19 passed (4 intentionally ignored)
- **gc_async_roots: 3 FAILED, 2 TIMEOUT** (hanging tests > 60s)

### Compiler Tests Summary
`cargo test -p ruyic`:
- **3 test files FAIL TO COMPILE:**
  - `tests/generics.rs:929` — syntax error (unclosed bracket/brace)
  - `tests/macro_expand.rs:206` — syntax error (unclosed bracket/brace)
  - `tests/diagnostics.rs` — references `ruyic::diagnostics` module which doesn't exist
- integration tests: 2/2 passed (test discovery only)

### .ry Type-Check Results
- `test/` dir: 6/8 passed, 2 failed (parse error in for_of.ry, type errors in impl_trait_builtin.ry)
- `crates/ruyic/tests/integration/cases/`: 32/45 passed, 13 failed
  - Includes expected failures for error-handling test cases
  - **generics.ry causes stack overflow** in compiler (SIGABRT)
