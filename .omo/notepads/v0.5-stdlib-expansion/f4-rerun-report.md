# F4 Scope Fidelity Check — v0.5-stdlib-expansion (Re-run)

**Date:** 2026-05-04
**Checker:** Sisyphus-Junior (F4)
**Previous Verdict:** REJECT (7 issues)
**Claim:** All 7 issues fixed

---

## Build Check

```
cargo check -p ruyi_runtime --no-default-features --lib
```
**Result:** PASS ✅

---

## Previous Issues Verification

### 1. T6 — regex.rs Box::into_raw + ruyi_regex_free
**Expected:** Switched to GC-compatible registry (no Box::into_raw, no ruyi_regex_free)
**Verification:**
- Read `crates/ruyi_runtime/src/regex.rs`
- Uses `static REGEX_REGISTRY: Lazy<Mutex<HashMap<i64, Regex>>>` with opaque i64 handles
- `ruyi_regex_compile` returns `register_regex(re) as *mut i8`
- No `Box::into_raw` found
- No `ruyi_regex_free` function exists
**Status:** RESOLVED ✅

---

### 2. T15 — __process_exec / __process_exec_with missing from registry
**Expected:** Added to compiler builtin registry / declarations
**Verification:**
- Runtime implementation exists in `crates/ruyi_runtime/src/process.rs` (lines 233, 254)
- stdlib/process.ry calls `__process_exec` (line 71) and `__process_exec_with` (line 81)
- Searched `crates/ruyic/src/codegen/builtins.rs` — **NO declarations for `ruyi_process_exec` or `ruyi_process_exec_with`**
- Searched entire `crates/ruyic/src` — **ZERO matches for `process_exec` or `regex_find`**
- The compiler has no knowledge of these builtins; any .ry code calling them will fail at link/codegen time
**Status:** NOT RESOLVED ❌

---

### 3. T26 — Iterator missing chain / enumerate / zip / sum / product
**Expected:** Added to `stdlib/collections.ry` Iterator trait
**Verification:**
- Read `stdlib/collections.ry` (529 lines)
- Iterator trait contains: `next`, `forEach`, `map`, `filter`, `reduce`
- Searched entire `stdlib/` for `fn chain`, `fn enumerate`, `fn zip`, `fn sum`, `fn product`
- **ZERO matches found**
**Status:** NOT RESOLVED ❌

---

### 4. T29 — regex.ry missing match()
**Expected:** Added `match()` method to regex.ry
**Verification:**
- Read `stdlib/regex.ry`
- Line 38: `fn match(self, text: string): Match? { ... }`
- Method body calls `__builtin_regex_find`
**Status:** PARTIALLY RESOLVED ⚠️ (see New Issue #1 below)

---

### 5. T33 — examples/v05_tests.ry missing
**Expected:** Created with @test demos
**Verification:**
- File exists at `examples/v05_tests.ry` (59 lines)
- Contains 5 @test functions: test_basic_assertion, test_assertEq, test_assertThrows, test_math_operations, test_string_operations
- Imports from "test" module
**Status:** RESOLVED ✅

---

### 6. T32 — core.ry + string.ry not merged
**Expected:** Verified zero duplicates, spec satisfied
**Verification:**
- Read `stdlib/core.ry` (module String with 10 methods)
- Read `stdlib/string.ry` (free functions with 25+ methods)
- **Duplicate names found:**
  - `split` — core.ry line 107, string.ry line 20
  - `startsWith` — core.ry line 89, string.ry line 39
  - `endsWith` — core.ry line 98, string.ry line 48
  - `contains` — core.ry line 80, string.ry line 57
  - `replace` — core.ry line 47, string.ry line 98
  - `toUpperCase` — core.ry line 55, string.ry line 153
  - `toLowerCase` — core.ry line 63, string.ry line 161
  - `trim` — core.ry line 71, string.ry line 177
  - `length` — core.ry line 17, string.ry line 169
  - `slice` — core.ry line 27, string.ry line 203
- Plan Definition of Done requires: "Merged core+string has zero duplicate String methods"
- The two files were **NOT merged** and **DUPLICATES EXIST**
**Status:** NOT RESOLVED ❌

---

### 7. T27 — Missing integration tests
**Expected:** buffer.ry, net.ry, regex.ry, test_attr.ry, test_framework.ry created
**Verification:**
- All 5 files exist in `crates/ruyic/tests/integration/cases/stdlib/`:
  - `buffer.ry` + `.expected` ✅
  - `net.ry` + `.expected` ✅
  - `regex.ry` + `.expected` ✅
  - `test_attr.ry` + `.expected` ✅
  - `test_framework.ry` + `.expected` ✅
**Status:** RESOLVED ✅

---

## New Issues Introduced by Fixes

### New Issue #1 — `match` keyword used as method name (regex.ry)
**File:** `stdlib/regex.ry:38`
**Code:** `fn match(self, text: string): Match? { ... }`
**Problem:** `match` is a reserved keyword in Ruyi (`Token::Match`). The lexer unconditionally maps `"match"` → `Token::Match` (`lexer/scanner.rs:445`). The parser's `parse_property_name()` only accepts `Token::Ident`, `Token::New`, `Token::SelfKw`, and `Token::String` — it does **not** accept `Token::Match` (`parser/parser.rs:722-741`).
**Impact:** This line will cause a parse error when regex.ry is compiled. No other file in the codebase uses `match` as a bare method name.
**Severity:** HIGH — stdlib module is syntactically invalid

### New Issue #2 — test.ry functions not exported
**File:** `stdlib/test.ry`
**Problem:** `assert`, `assertEq`, `assertThrows`, `describe`, `it` are defined but have no `export` statements. Both `examples/v05_tests.ry` and `test_framework.ry` import them with `import { ... } from "test"`.
**Note:** Other stdlib files (io.ry, path.ry, collections.ry) also lack exports, so this may be a systemic issue rather than a new one introduced by these fixes. However, it means the newly added test framework files likely cannot compile.
**Severity:** MEDIUM (if exports are required by the module system)

---

## Summary

| Issue | Status |
|-------|--------|
| T6 (regex.rs GC registry) | ✅ RESOLVED |
| T15 (__process_exec in registry) | ❌ NOT RESOLVED |
| T26 (Iterator methods) | ❌ NOT RESOLVED |
| T29 (regex.ry match()) | ⚠️ PARTIAL (syntax error) |
| T33 (v05_tests.ry) | ✅ RESOLVED |
| T32 (core+string merge) | ❌ NOT RESOLVED |
| T27 (integration tests) | ✅ RESOLVED |

**Previous Issues Resolved:** 3 / 7 (with T29 being a partial fix that introduces a new syntax error)
**New Issues Introduced:** 1 confirmed (match keyword), 1 suspected (missing exports)

---

## Verdict

```
Previous Issues [3/7 resolved] | New Issues [1/N]
VERDICT: REJECT
```

**Rationale:**
- Three of the seven claimed fixes (T15, T26, T32) are objectively not resolved.
- T29's fix introduces a new, high-severity syntax error by using the reserved keyword `match` as a method name.
- The compiler-side builtin wiring (registry/declarations) for `__process_exec`, `__process_exec_with`, `__builtin_regex_find`, and all other Wave 2–5 builtins remains completely missing — no BuiltinRegistry infrastructure exists in `crates/ruyic/src`.
