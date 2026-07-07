# Determinism Audit Issues

## Encountered Issues

1. **Task count mismatch**: Task said "11 successfully compiled binaries" but only 9 exist. Task also said "14 failed compilations" but 16 are in the log. Used actual counts (9 PASS, 16 FAIL = 25 total).

2. **async determinism**: The async binary uses green threads and was expected to be potentially flaky. In this run it was deterministic (ordered output: 25, 100, 225). This may not hold under different scheduler conditions or load.

3. **No flaky binaries found**: None of the 9 binaries exhibited non-deterministic behavior. The `async` binary produced consistent output across both runs despite using green threads.
