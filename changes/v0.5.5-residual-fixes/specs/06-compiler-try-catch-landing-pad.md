# Spec: Try/Catch Landing Pad Code Generation

## MODIFIED Requirements

### REQ-LPAD-001: Code generation MUST use LLVM `invoke` in try bodies

For every function call emitted inside a `try` block body, the compiler MUST generate an LLVM `invoke` instruction (with a normal-success basic block and an unwind basic block) instead of `call`. This routes exceptions thrown by the callee into the `try`'s landing pad.

This spec subsumes and finalizes `fix-try-catch-invoke` (state: closing) which only partially addressed this requirement.

#### Scenario: Exception thrown from inner function caught by outer try
- **WHEN** source contains `try { innerThrow(); } catch (e) { print("caught"); }` and `innerThrow()` actually throws
- **THEN** the program prints `caught` (not crash or unhandled exception)

#### Scenario: Function call outside try remains call instruction
- **WHEN** codegen visits `foo()` outside any `try` block
- **THEN** the generated IR still uses `call %foo()` (zero regression)

### REQ-LPAD-002: Catch block MUST contain a `landingpad` instruction

For each `try` block, the LLVM IR MUST contain a `landingpad` instruction in the catch block that returns an exception pointer, and a selector-based dispatch to the appropriate catch arm.

#### Scenario: Multiple catch arms dispatched via selector
- **WHEN** source contains `try { ... } catch (e: ErrorA) { ... } catch (e: ErrorB) { ... }` and `ErrorA` is thrown
- **THEN** the catch arm for `ErrorA` executes (not `ErrorB`)

### REQ-LPAD-003: All 13 `#[ignore]` try_catch_invoke tests MUST pass

The 13 `#[ignore]` tests in `crates/ruyic/tests/try_catch_invoke.rs` MUST all pass after this spec is implemented.

#### Scenario: cargo test --test try_catch_invoke -- --ignored passes all
- **WHEN** `cargo test -p ruyic --test try_catch_invoke -- --ignored --test-threads=1` is executed with LLVM 14 available
- **THEN** all 13 tests pass (up from baseline of 0 passing)

### REQ-LPAD-004: All 3 `#[ignore]` compilation_throw_unreachable tests MUST pass

The 3 `#[ignore]` tests in `crates/ruyic/tests/compilation_throw_unreachable.rs` MUST all pass after this spec is implemented.

#### Scenario: throw followed by unreachable produces valid IR
- **WHEN** source contains `throw new Error();` as the last statement of a function
- **THEN** the generated IR contains `unreachable` after `call ruyi_throw`, and cargo test compilation_throw_unreachable passes