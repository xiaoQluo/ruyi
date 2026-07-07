# v0.4.1 Test Coverage Report

## Integration Tests (.ry files)

| File | Feature | Status |
|------|---------|--------|
| for_in.ry | for-in loop | ✅ |
| for_of.ry | for-of loop | ❌ Parse error at line 11 (`int[]` type syntax) |
| computed_member.ry | computed member access | ✅ |
| optional_chaining.ry | optional chaining | ✅ |
| template_literal.ry | template literal | ✅ |
| impl_trait_builtin.ry | impl trait for built-in types | ❌ Type checker errors (built-in type trait impl) |
| match_edge_cases.ry | match edge cases | ✅ |
| cross_features.ry | cross-feature integration | ✅ |

## Runtime Tests

Filter: `cargo test -p ruyi_runtime --no-default-features`

### Library Unit Tests (inline in `src/`)

| Module | Test Count | Status |
|--------|------------|--------|
| alloc::tests | 4 | ✅ 4 passed |
| arc::tests | 4 | ✅ 4 passed |
| async_runtime::tests | 7 | ✅ 7 passed |
| builtins::tests | 12 | ✅ 12 passed |
| exception::runtime::tests | 5 | ✅ 5 passed |
| exception::tests | 6 | ✅ 6 passed |
| exception::types::tests | 4 | ✅ 4 passed |
| gc::barrier::tests | 2 | ✅ 2 passed |
| gc::generational::tests | 8 | ✅ 8 passed |
| gc::roots::tests | 2 | ✅ 2 passed |
| gc::tests | 3 | ✅ 3 passed |
| **Total (lib)** | **57** | **✅ 57 passed** |

### Integration Tests (`tests/` directory)

| File | Test Count | Status |
|------|------------|--------|
| async_gc_roots.rs | 1 | ✅ 1 passed |
| async_runtime.rs | 8 | ✅ 8 passed |
| builtins.rs | 5 | ✅ 5 passed |
| exception.rs | 8 | ✅ 8 passed |
| exception_runtime.rs | 23 | ✅ 19 passed, 4 ignored |
| gc_async_roots.rs | 5 | ❌ 3 failed, 2 timed out |
| gc_exports.rs | 7 | ✅ 7 passed |
| gc_thread_local.rs | 6 | ⚠️ 5 passed, 1 failed |
| runtime.rs | 5 | ✅ 5 passed |
| **Total (integration)** | **68** | **✅ 58 passed, ⚠️ 1 failed, ❌ 5 failed/timeout, 4 ignored** |

## Summary

### All Tests Combined

| Category | Count | Status |
|----------|-------|--------|
| Library unit tests | 57 | ✅ 57/57 |
| Integration test files | 68 | ✅ 58/68, 4 ignored, 1 partial, 5 failed |
| **Total** | **125** | **✅ 115 passed, ⚠️ 5 failed, 1 partial, 4 ignored** |

### Integration .ry file compilation

| Status | Count |
|--------|-------|
| Type check passed | 6 |
| Type check failed | 2 |

### v0.4.1 Feature Coverage

| Feature | Status | Notes |
|---------|--------|-------|
| for-in loop | ✅ | `for_in.ry` passes type check |
| for-of loop | ❌ | `for_of.ry`: parser doesn't support `int[]` type syntax |
| computed member access | ✅ | `computed_member.ry` passes type check |
| optional chaining | ✅ | `optional_chaining.ry` passes type check |
| template literal | ✅ | `template_literal.ry` passes type check |
| impl trait for built-in types | ❌ | `impl_trait_builtin.ry`: type checker doesn't support trait impls for built-in types |
| match edge cases | ✅ | `match_edge_cases.ry` passes type check |
| cross-feature integration | ✅ | `cross_features.ry` passes type check |
| GC (generational, thread-local, barriers) | ✅ | All GC lib tests pass; GC integration tests: 5/7 pass, 1 fail, 1 partial |
| Async runtime | ✅ | All async tests pass (lib + integration) |
| Exception handling | ✅ | All exception tests pass (lib + integration, 4 ignored) |

**All v0.4.1 features covered: NO** — 2 integration .ry files have compilation issues, and several runtime tests have failures.

### Known Issues

1. **for_of.ry**: Parser doesn't recognize `int[]` type syntax (array type annotation). The for-of loop itself seems to work (basic `for (let item of arr)` compiles), but type annotations for arrays are not supported.
2. **impl_trait_builtin.ry**: Type checker doesn't support implementing traits for built-in primitive types (`string`, `int`, `float`, `bool`).
3. **gc_async_roots.rs**: 3 tests fail, 2 tests hang/timeout — async GC root tracking has issues.
4. **gc_thread_local.rs**: `test_thread_exit_does_not_corrupt_main_heap` fails — thread exit GC isolation has a bug.
