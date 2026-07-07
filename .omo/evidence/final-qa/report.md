# Ruyi Examples Test Report

**Generated:** 2026-05-04  
**Compiler:** ruyic 0.5.0  
**OS:** Darwin MacBook-Pro.local 25.4.0 (Darwin Kernel Version 25.4.0, x86_64)  
**LLVM:** Pre-built binary (LLVM_SYS_140_PREFIX not set)

---

## Summary

| Status | Count |
|--------|-------|
| PASS | 9 |
| FAIL | 16 |
| FLAKY | 0 |
| TIMEOUT | 0 |
| SKIP_SILENT | 0 |
| **Total** | **25** |

**Pass rate:** 36% (9/25)

---

## Per-File Results

| # | Status | File | Notes |
|---|--------|------|-------|
| 1 | PASS | hello.ry | Output matches baseline |
| 2 | PASS | fibonacci.ry | Output matches baseline |
| 3 | PASS | float_math.ry | Output matches baseline |
| 4 | PASS | compare_test.ry | Output matches baseline |
| 5 | PASS | ternary.ry | Output matches baseline |
| 6 | PASS | array.ry | Output matches baseline |
| 7 | PASS | async.ry | Output matches baseline |
| 8 | PASS | generics.ry | Output matches baseline |
| 9 | PASS | generics_simple.ry | Output matches baseline |
| 10 | FAIL | control_flow.ry | Parse error |
| 11 | FAIL | functions.ry | Parse error |
| 12 | FAIL | variables_and_types.ry | Codegen error |
| 13 | FAIL | try_catch.ry | Linker error |
| 14 | FAIL | error_handling.ry | Parse error |
| 15 | FAIL | type_system.ry | Parse error |
| 16 | FAIL | generics_comprehensive.ry | Parse error |
| 17 | FAIL | classes_and_objects.ry | Codegen error |
| 18 | FAIL | traits.ry | Type mismatch |
| 19 | FAIL | pattern_matching.ry | Parse error |
| 20 | FAIL | async_comprehensive.ry | Parse error |
| 21 | FAIL | v04_minimal.ry | Codegen error |
| 22 | FAIL | v04_simple.ry | Codegen error |
| 23 | FAIL | v04_features.ry | Codegen error |
| 24 | FAIL | v05_demo.ry | Unknown variables |
| 25 | FAIL | v05_tests.ry | Missing CLI flag |

---

## Failure Details

### Parse Errors (7 files)

#### control_flow.ry
- **Error:** Expected identifier but found `keyword 'from'` at line 57, column 19
- **Category:** Parser does not recognize `from` keyword in this context

#### functions.ry
- **Error:** Expected `=>` but found `:` at line 101, column 23
- **Category:** Parser expects arrow syntax for function type, found colon

#### error_handling.ry
- **Error:** Unexpected token `keyword 'catch'` at line 143, column 5
- **Category:** Parser does not recognize `catch` keyword at this position

#### type_system.ry
- **Error:** Expected type at line 25, column 18
- **Category:** Parser failed to parse type annotation

#### generics_comprehensive.ry
- **Error:** Expected `;` but found `!` at line 50, column 22
- **Category:** Parser does not handle null assertion operator `!` in this context

#### pattern_matching.ry
- **Error:** Expected `;` but found `}` at line 176, column 26
- **Category:** Parser expects semicolon before closing brace

#### async_comprehensive.ry
- **Error:** Expected `fn` after `async` at line 52, column 21
- **Category:** Parser does not recognize `async` without immediate `fn` keyword

### Codegen Errors (5 files)

#### variables_and_types.ry
- **Error:** Unsupported binary operator: Power
- **Category:** Code generator lacks support for power/exponentiation operator

#### classes_and_objects.ry
- **Error:** Unsupported binary operator: Power
- **Category:** Code generator lacks support for power/exponentiation operator

#### v04_minimal.ry
- **Error:** Invalid operands for `+`
- **Category:** Code generator rejects operand types for addition

#### v04_simple.ry
- **Error:** Invalid operands for `+`
- **Category:** Code generator rejects operand types for addition

#### v04_features.ry
- **Error:** Invalid operands for `+`
- **Category:** Code generator rejects operand types for addition

### Linker Errors (1 file)

#### try_catch.ry
- **Error:** Undefined symbols for architecture x86_64: `___gxx_personality_v0`
- **Category:** Missing C++ exception personality function. Linker cannot find `libstdc++` or `libc++` exception support.

### Type Errors (1 file)

#### traits.ry
- **Error:** Type mismatch: expected `dyn Printable`, but found `string` / `int`
- **Category:** Trait dispatch does not accept concrete types where `dyn Trait` is expected

### Unknown Variables (1 file)

#### v05_demo.ry
- **Error:** Unknown variable: `Timestamp` / `Random`
- **Category:** Standard library functions `Timestamp` and `Random` are not defined

### Missing CLI Flag (1 file)

#### v05_tests.ry
- **Error:** Unexpected argument `--test` found
- **Category:** Compiler CLI does not support `--test` flag

---

## Failure Categories Summary

| Category | Count | Files |
|----------|-------|-------|
| Parse Error | 7 | control_flow, functions, error_handling, type_system, generics_comprehensive, pattern_matching, async_comprehensive |
| Codegen Error | 5 | variables_and_types, classes_and_objects, v04_minimal, v04_simple, v04_features |
| Linker Error | 1 | try_catch |
| Type Error | 1 | traits |
| Unknown Variable | 1 | v05_demo |
| Missing CLI Flag | 1 | v05_tests |

---

## Environment

| Property | Value |
|----------|-------|
| Compiler | ruyic 0.5.0 |
| LLVM | Pre-built binary (LLVM_SYS_140_PREFIX not set) |
| OS | macOS 25.4.0 (Darwin 25.4.0, x86_64) |
| Architecture | x86_64 |
| Date | 2026-05-04 |
