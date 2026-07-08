# Pending Issues - v0.5 Legacy

> Generated from Oracle Phase 1 review (2026-05-17)
> These are pre-existing issues from v0.5, NOT introduced by Phase 1 fixes.

---

## Test Failures

### 1. `test_check_match_statement`
- **File**: `crates/ruyic/src/typechecker/checker.rs:172`
- **Test**: `assert!(!check_program("match (1) { 1 => { } }").has_errors);`
- **Issue**: v0.5 match/codegen changes caused type checker to report new constraints on match statements
- **Note**: Test comment mentions "unknown variable 'x'" but code has no `x` - copy-paste error
- **Blame**: `fc58318` (2026-05-02, v0.4.1)
- **Fix**: Update test to match v0.5 type checker behavior or fix match statement handling

### 2. `test_bool_patterns_with_wildcard`
- **File**: `crates/ruyic/src/typechecker/patterns.rs:266`
- **Test**: `assert!(result.has_redundancy);` for arms `[true, _]`
- **Issue**: Test assertion is logically wrong - `true` matches true, `_` matches false, both arms reachable, NO redundancy
- **Fix**: Change assertion to `assert!(!result.has_redundancy);`

### 3. `test_from_annotation_generic`
- **File**: `crates/ruyic/src/typechecker/types.rs:652`
- **Test**: Expects `Generic{base:"Array", args:[Int]}` for `Array<int>`
- **Issue**: v0.5 added special handling `Array<T>` → `Type::Array(T)` at commit `764676a`
- **Fix**: Update expected value to `Type::Array(Box::new(Type::Int))`

---

## Technical Debt

### 4. `allow_partial_codegen` Scope
- **File**: `crates/ruyic/src/driver.rs:583`
- **Issue**: Flag set to `true` globally, silently swallowing codegen errors for user code (not just stdlib)
- **Priority**: High (Phase 2)
- **Fix**: Scope to stdlib-only or add visible diagnostics for user code

### 5. Empty `impl RenderSeverity {}` Block
- **File**: `crates/ruyic/src/diagnostics/render.rs:70`
- **Issue**: Empty impl block after removing `prefix()` method
- **Fix**: Remove empty impl block

### 6. Error Code Documentation Outdated
- **File**: `crates/ruyic/src/diagnostics/codes.rs:5-10`
- **Issue**: Comments still reference "E1xxx/E2xxx/E3xxx/E4xxx/W1xxx" format
- **Fix**: Update to reflect new "E/W + number" format

---

## Status Legend
- 🔴 Blocking: Prevents release
- 🟡 High: Should fix soon
- 🟢 Low: Cosmetic/minor
