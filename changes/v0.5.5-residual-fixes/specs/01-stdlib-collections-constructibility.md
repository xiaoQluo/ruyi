# Spec: stdlib/collections.ry RangeError/ArrayIterator Constructibility (T9 收尾)

## MODIFIED Requirements

### REQ-COLL-001: `RangeError` MUST be constructible as a named type

The `RangeError` class in `stdlib/collections.ry` MUST be callable as a constructor: `throw RangeError("message")` MUST compile and instantiate an exception object at runtime.

Previously, T9 (`809e6c9`) made `RangeError` a Named type but did not enable constructor callability, so `throw RangeError(...)` aborted compilation.

#### Scenario: Throw RangeError compiles successfully
- **WHEN** the source contains `throw RangeError("index out of bounds")`
- **THEN** the program compiles without "type RangeError is not constructible" error and runs correctly

#### Scenario: Catch RangeError matches catch arm
- **WHEN** `throw RangeError("...")` is wrapped in `try { ... } catch (e: RangeError) { ... }`
- **THEN** the exception is caught by the RangeError arm

### REQ-COLL-002: `ArrayIterator` MUST be constructible as a named type

The `ArrayIterator` class in `stdlib/collections.ry` MUST be callable as a constructor: `let iter = ArrayIterator(arr)` MUST compile and produce an iterator object usable with `.next()`.

#### Scenario: ArrayIterator instantiation compiles and runs
- **WHEN** the source contains `let iter = ArrayIterator(myArray); while (iter.hasNext()) { print(iter.next()); }`
- **THEN** the program compiles and iterates over all elements of `myArray`

### REQ-COLL-003: All 21 `#[ignore]` codegen tests referencing RangeError/ArrayIterator MUST pass

The 21 codegen tests currently FAIL because they instantiate `RangeError` / `ArrayIterator` (or types depending on them) MUST all pass after this spec is implemented.

#### Scenario: cargo test --test codegen -- --ignored passes 21 tests
- **WHEN** `cargo test -p ruyic --test codegen -- --ignored --test-threads=1` is executed with LLVM 14 available
- **THEN** at least 21 tests that previously FAIL now pass

### REQ-COLL-004: All 8 stdlib modules MUST be audited for correctness

All 8 stdlib modules (`array`, `collections`, `map`, `set`, `string`, `math`, `time`, `json`) MUST be scanned for:
- Type signatures consistent with usage in examples
- Function bodies implement documented behavior
- No dead code or stubs without TODO

Note: `math`, `time`, `json` are NOT in scope to be implemented; only their completeness is audited.

#### Scenario: stdlib audit report produced
- **WHEN** `cargo run --bin audit-stdlib` is executed (new audit tooling)
- **THEN** a markdown report lists each module's audit results (pass/warn/fail per function)