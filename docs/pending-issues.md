# Pending Issues - v0.5 Legacy

> Generated from Oracle Phase 1 review (2026-05-17)
> Last updated: 2026-07-16 (v0.5.9 audit)
> These are pre-existing issues from v0.5, NOT introduced by Phase 1 fixes.

---

## Test Failures

### 1. `test_check_match_statement` — ✅ FIXED (v0.5.x)
- **File**: `crates/ruyic/src/typechecker/checker.rs:206`
- **Status**: Test passes as of v0.5.9 (`cargo test -p ruyic --lib -- test_check_match_statement` → passed)
- **Original Issue**: v0.5 match/codegen changes caused type checker to report new constraints on match statements
- **Resolution**: Type checker behavior aligned with match statement expectations in subsequent versions

### 2. `test_bool_patterns_with_wildcard` — ✅ FIXED (v0.5.x)
- **File**: `crates/ruyic/src/typechecker/patterns.rs:343`
- **Status**: Test passes as of v0.5.9 (`cargo test -p ruyic --lib -- test_bool_patterns_with_wildcard` → passed)
- **Original Issue**: Test assertion was logically wrong (`assert!(result.has_redundancy)` → `assert!(!result.has_redundancy)`)
- **Resolution**: Assertion corrected; both arms (`true` and `_`) are correctly identified as non-redundant

### 3. `test_from_annotation_generic` — ✅ FIXED (v0.5.x)
- **File**: `crates/ruyic/src/typechecker/types.rs:740`
- **Status**: Test passes as of v0.5.9 (`cargo test -p ruyic --lib -- test_from_annotation_generic` → passed)
- **Original Issue**: Expected `Generic{base:"Array", args:[Int]}` but v0.5 normalizes to `Type::Array(T)`
- **Resolution**: Expected value updated to `Type::Array(Box::new(Type::Int))`

---

## Technical Debt

### 4. `allow_partial_codegen` Scope — 🟡 IN PROGRESS (fix-batch-low-risk-defects)
- **File**: `crates/ruyic/src/driver.rs:583`
- **Issue**: Flag set to `true` globally, silently swallowing codegen errors for user code (not just stdlib)
- **Priority**: High
- **Fix**: Scoping to stdlib-only via `stdlib_item_count` in `fix-batch-low-risk-defects` change

### 5. Empty `impl RenderSeverity {}` Block
- **File**: `crates/ruyic/src/diagnostics/render.rs:70`
- **Issue**: Empty impl block after removing `prefix()` method
- **Fix**: Remove empty impl block

### 6. Error Code Documentation Outdated
- **File**: `crates/ruyic/src/diagnostics/codes.rs:5-10`
- **Issue**: Comments still reference "E1xxx/E2xxx/E3xxx/E4xxx/W1xxx" format
- **Fix**: Update to new "E/W + number" format

---

## Runtime / stdlib Defects (v0.5.11 audit)

### 7. `Once.do()` stdlib stub always returns false — 🟡
- **File**: `stdlib/sync.ry` (`Once` class)
- **Issue**: `do()` is a stub returning `false`; `isCompleted()` therefore never becomes true. Exposed by `examples/concurrency/sync_demo.ry` section "Once".
- **Fix**: Implement `__once_*` runtime FFI backed by `std::sync::Once`, or a Mutex+bool guard closure pair.

### 8. `ThreadLocal.load()` dyn vs string `===` comparison — 🟡
- **File**: `stdlib/thread_local.ry`
- **Issue**: `load()` returns `dyn`; comparing the result with a string literal via `===` is always false (dynamic comparison does not unwrap string payloads). Exposed by `sync_demo.ry` section "ThreadLocal".
- **Fix**: Teach dynamic `===` to compare Dynamic-wrapped strings, or give `ThreadLocal` a typed accessor.

### 9. comprehensive_demo crypto sections exceed 300 s — 🟢
- **File**: `examples/comprehensive/comprehensive_demo.ry` (section 4 crypto)
- **Issue**: PBKDF2 passes (~2–4 min, pure compute) but the following `cryptoRandomHex` (CSPRNG) section times out a 300 s budget. Known performance characteristic of the crypto FFI path, not a codegen regression.
- **Fix**: Reduce demo iteration counts or speed up the CSPRNG FFI.

### 10. `ruyi_obj_keys` returns corrupt key strings for 2+ field objects — 🟡 (pre-existing at 86a7898)
- **File**: `crates/ruyi_runtime/src/builtins.rs:407` (`ruyi_obj_keys`), test `crates/ruyi_runtime/tests/obj_keys.rs::test_keys_of_2_field_object`
- **Issue**: `Object.keys()` on a 2-field object yields a key that fails UTF-8 validation (`valid_up_to: 2`) — the array slot does not round-trip the CString. Fails identically on the HEAD baseline worktree.
- **Fix**: Audit `object_field_map` key ownership vs `ruyi_array_set` slot writes (lifetime/copy semantics of the key pointers).

---

## Status Legend
- 🔴 Blocking: Prevents release
- 🟡 High: Should fix soon
- 🟢 Low: Cosmetic/minor
- ✅ FIXED: Resolved in current or prior version
