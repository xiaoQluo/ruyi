# Spec: Try/Catch Exception Routing (build_invoke + landing pad)

## MODIFIED Requirements

### REQ-TCI-001: `try/catch` MUST catch exceptions thrown from called functions

When code inside a `try` block invokes a function that throws (via `throw` statement or `ruyi_throw`), the exception MUST propagate into the matching `catch` block of the enclosing `try`, rather than bypassing `try/catch` entirely.

The compiler MUST generate LLVM `invoke` instructions for function calls inside a `try` region (with the normal-success basic block and the unwind basic block), so LLVM's exception handling semantics can route the exception to the catch handler.

#### Scenario: Exception thrown from inner function is caught by outer try/catch
- **WHEN** the source contains a `try { innerThrow(); } catch (e) { print("caught"); }` and `innerThrow` invokes `throw new Error("boom")` (a non-trivial call chain)
- **THEN** the program compiles, the LLVM IR contains `invoke` instructions for `innerThrow` (NOT `call`), the program runs, and the output contains `caught`

#### Scenario: Exception thrown directly inside try body still caught (regression guard)
- **WHEN** the source contains `try { throw new Error("boom"); } catch (e) { print("caught"); }` (the throw is in the same function as the try)
- **THEN** the program compiles and prints `caught` (existing behavior preserved)

#### Scenario: Exception not matching any catch block propagates upward
- **WHEN** the source contains a `try { innerThrow(); }` with no catch block, nested inside another `try { ... } catch (e) { print("outer"); }`
- **THEN** the inner `try` re-throws via `ruyi_throw`, and the outer `try`'s catch receives it

### REQ-TCI-002: `throw` statements MUST terminate the current basic block

After invoking the `ruyi_throw` runtime function (which is `noreturn`), the compiler MUST emit an LLVM `unreachable` instruction so the basic block after the throw is correctly terminated. This prevents PHI node inconsistencies in the catch block.

#### Scenario: catch block PHI reflects no value from throw path
- **WHEN** the IR for a function containing `try { let x = compute(); throw new Error(); } catch (e) { use(x); }` is inspected
- **THEN** the `unreachable` instruction follows the `call ruyi_throw`, and the catch block's PHI for `x` has no incoming edge from the throw path

#### Scenario: try_stack mechanism still branches to catch/finally after throw
- **WHEN** the source contains `try { throw new Error(); } catch (e) { ... }`
- **THEN** the control flow branches from the throw call to the catch's landing-pad block (continuing existing behavior)

### REQ-TCI-003: `compile_call` MUST respect `try` context for invoke vs call selection

The `compile_call` codegen function MUST check whether the current emission context is inside a `try` block (tracked via `CodegenContext`). When inside a `try`, the function call MUST be emitted as an LLVM `invoke` instruction with a designated unwind basic block; otherwise the existing `build_call` is used (preserving performance for non-try calls).

#### Scenario: Function call inside try produces invoke instruction
- **WHEN** codegen visits `foo()` inside a `try { ... }` block
- **THEN** the generated IR uses `invoke %foo() to label %normal_bb unwind label %catch_bb` (not `call`)

#### Scenario: Function call outside try remains call instruction
- **WHEN** codegen visits `foo()` outside any `try` block
- **THEN** the generated IR still uses `call %foo()` (zero regression)

#### Scenario: Function call in nested try uses innermost catch block as unwind target
- **WHEN** codegen visits `foo()` inside a nested `try { try { foo(); } catch (e) { ... } } catch (outer) { ... }`
- **THEN** the `invoke` instruction's unwind target is the inner catch's landing pad (not the outer one)

### REQ-TCI-004: CodegenContext MUST track try-block context

`CodegenContext` MUST track the current "inside-try" state so that `compile_call` and other callees can decide whether to emit `invoke` or `call`. The state MUST be a stack (not a single boolean) to correctly handle nested try/catch blocks. Entering `compile_try` pushes a frame; exiting (on normal completion, catch, or throw) pops it.

#### Scenario: Nested try blocks maintain a stack of try contexts
- **WHEN** codegen enters `try { try { ... } catch (a) { ... } } catch (b) { ... }`
- **THEN** the context push/pop is balanced: the outer catch pop occurs after the inner try frame is popped, with `try_stack.len() == 0` after the outer try exits

#### Scenario: Function call after leaving a try block uses call, not invoke
- **WHEN** codegen finishes a `try { ... }` block and emits a subsequent function call in the same function
- **THEN** the subsequent call uses `build_call`, not `invoke` (confirming try-stack was popped)

### REQ-TCI-005: LLVM IR for try/catch MUST include landingpad + catch dispatch

For each `try` block, the LLVM IR MUST contain:
- One or more `invoke` instructions in the try body (REQ-TCI-001)
- A `landingpad` instruction in the catch block, returning an exception pointer
- A selector-based dispatch to the appropriate catch arm (using `LandingPadGenerator::build_catch_dispatch`)
- A `resume` instruction (or equivalent) if an exception is uncaught by any catch arm

#### Scenario: Catch dispatcher matches multiple catch arms via selector
- **WHEN** the source contains `try { ... } catch (e: ErrorA) { print("A"); } catch (e: ErrorB) { print("B"); }` and a function throws `ErrorA`
- **THEN** the IR contains exactly one `landingpad` with selector-based branch matching the thrown type to the first catch arm, and the program prints `A`

#### Scenario: finally block executes on both normal and exceptional paths
- **WHEN** the source contains `try { ... } catch (e) { ... } finally { clean(); }`
- **THEN** the IR contains branches to the finally block from both the normal-success and the catch paths, the finally executes exactly once per try invocation (regression guard for existing finally behavior)

### REQ-TCI-006: Codegen integration test for try/catch MUST verify invoke instruction

A new codegen integration test (`#[ignore]`d, requires LLVM 14) MUST verify that compiling `try { innerThrow(); } catch (e) { print("caught"); }` produces an LLVM IR file containing `invoke` instructions inside the try block and a `landingpad` in the catch block.

#### Scenario: Compiled .ry file contains invoke in try body
- **WHEN** the codegen integration test compiles `examples/try_catch_invoke.ry` with `--emit-llvm`
- **THEN** the IR file contains at least one `invoke` instruction (and a corresponding `landingpad`)

### REQ-TCI-007: `examples/try_catch_invoke.ry` MUST demonstrate end-to-end correctness

A new Ruyi example MUST demonstrate that an exception thrown from a called function is caught by an enclosing `try/catch`:

```ruyi
fn innerThrow(): void {
  throw new Error("boom");
}

fn main(): int {
  try {
    innerThrow();
  } catch (e) {
    print("caught");
    return 0;
  }
  return 1;
}
```

This example MUST be added to `examples/run_examples.sh` and MUST increase the total example count from 33 to 34.

#### Scenario: Example compiles and runs successfully
- **WHEN** `examples/try_catch_invoke.ry` is compiled with `ruyic` and run
- **THEN** the program exits with status 0 and prints `caught` (proving the exception from `innerThrow` reached the outer catch)

#### Scenario: Example increases run_examples.sh total
- **WHEN** `bash examples/run_examples.sh` is executed
- **THEN** the output shows `Total: 34 | Passed: 34 | Failed: 0`
