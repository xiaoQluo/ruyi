# Determinism Audit Learnings

## Summary
- All 9 successfully compiled binaries are fully deterministic (identical output on 2 runs)
- 16 examples fail to compile (logged in `examples/target/failures.log`)

## Key Findings

### Compiled & PASS (9)
| Binary | Exit Code | Status |
|--------|-----------|--------|
| hello | 0 | PASS |
| fibonacci | 0 | PASS |
| float_math | 0 | PASS |
| compare_test | 0 | PASS |
| ternary | 0 | PASS |
| array | 0 | PASS |
| async | 0 | PASS |
| generics | 0 | PASS |
| generics_simple | 0 | PASS |

### Flakiness Assessment
- **async**: Expected to be potentially flaky (green threads) but actually deterministic — outputs 25, 100, 225 in consistent order
- **float_math**: No precision differences between runs
- All others: Simple deterministic programs, no randomness sources

### Compilation Failures (16)
Root causes identified:
- **Parse errors** (6): control_flow, functions, error_handling, type_system, generics_comprehensive, pattern_matching, async_comprehensive — language features not yet implemented
- **Codegen errors** (4): variables_and_types, classes_and_objects (Power operator), v04_minimal/simple/features (Invalid operands for +)
- **Linker errors** (1): try_catch (C++ exception support missing)
- **Type errors** (1): traits (dyn Printable mismatch)
- **Other** (2): v05_demo (Unknown variable), v05_tests (unexpected --test argument)

## Files Created
- `examples/target/*.expected` — 9 baseline output files
- `examples/target/baselines.json` — Master status file (25 entries)
