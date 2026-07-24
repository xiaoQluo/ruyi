# Spec: exception-unwinder

## MODIFIED Requirements

### R1: throw_exception SHALL invoke the LLVM unwinder

`throw_exception(exc: RuyiException)` SHALL NOT use `panic!()`. It SHALL construct an
`UnwindException` (per Itanium C++ ABI) with the Ruyi exception payload and invoke
`_Unwind_RaiseException` to initiate stack unwinding.

##### Scenario: throw_exception with Error type

- **WHEN** `throw_exception` is called with a `RuyiException` of type `ERROR`
- **THEN** an `UnwindException` SHALL be allocated with `KLANG_EXCEPTION_CLASS`
- **AND** `_Unwind_RaiseException` SHALL be called with the allocated exception pointer
- **AND** the function SHALL NOT return (it is `-> !`)

##### Scenario: _Unwind_RaiseException returns

- **WHEN** `_Unwind_RaiseException` returns `_URC_END_OF_STACK` (no handler found)
- **THEN** the program SHALL call `ruyi_exception_cleanup` to deallocate the exception
- **AND** the program SHALL abort (as there is no handler for the exception)

---

### R2: C FFI ruyi_throw SHALL trigger unwinding

The `#[no_mangle] pub extern "C" fn ruyi_throw(msg: *const i8)` in `c_exports.rs`
SHALL no longer silently store a pending exception pointer. It SHALL construct a
`RuyiException` from the message pointer and invoke `throw_exception` to trigger
proper stack unwinding.

##### Scenario: ruyi_throw called from compiled code

- **WHEN** a compiled Ruyi program calls `ruyi_throw` with a message pointer
- **THEN** a `RuyiException` SHALL be constructed with `type_id = ERROR` and the message
- **AND** `throw_exception` SHALL be called
- **AND** if an active try block exists, the landing pad SHALL catch the exception

##### Scenario: ruyi_throw outside try block

- **WHEN** `ruyi_throw` is called and no try block exists on the call stack
- **THEN** the program SHALL terminate via the unwinder's default termination handler
- **AND** no Rust panic backtrace SHALL be emitted

---

### R3: Exception::throw (compiler side) SHALL align with runtime

The `Exception::throw()` method in `ruyic/src/runtime/exception.rs` SHALL NOT use `panic!()`.
It SHALL call the runtime's `ruyi_throw` FFI function or equivalent unwinder invocation.

##### Scenario: Compiler-generated throw code

- **WHEN** the codegen emits a call to `ruyi_throw` for a `throw` expression
- **THEN** the compiled binary SHALL invoke unwinding rather than Rust panic
- **AND** the exception SHALL be catchable by enclosing try blocks

---

### R4: try/catch SHALL catch thrown exceptions end-to-end

A compiled Ruyi program using `try { throw expr; } catch (e) { handler; }` SHALL:
- Execute `throw expr` → invoke the unwinder
- Transfer control to the catch block's landing pad
- Bind the caught exception to the catch variable `e`
- Execute the catch block body
- Continue execution after the try/catch construct

##### Scenario: Basic throw → catch

- **WHEN** a compiled Ruyi program contains:
  ```
  try { throw Error.new("test error"); }
  catch (e: Error) { print(e.getMessage()); }
  ```
- **THEN** `"test error"` SHALL be printed
- **AND** execution SHALL continue after the catch block
- **AND** the program SHALL exit with code 0

##### Scenario: Nested try-catch with inner rethrow

- **WHEN** an inner try block throws and inner catch rethrows
- **THEN** the outer catch SHALL capture the rethrown exception
- **AND** the outer catch handler SHALL execute

##### Scenario: try-catch-finally

- **WHEN** a try block throws and a finally block exists
- **THEN** the finally block SHALL execute BEFORE the catch handler
- **AND** if the catch also throws, finally SHALL still execute

##### Scenario: Uncaught exception

- **WHEN** a `throw` expression is executed outside any try block
- **THEN** the program SHALL terminate with a non-zero exit code
- **AND** the exception message SHALL be printed to stderr

---

### R5: Existing codegen and runtime tests SHALL pass without regression

##### Scenario: exception_runtime tests

- **WHEN** running `cargo test -p ruyi_runtime --test exception_runtime`
- **THEN** all tests SHALL pass, including 4 previously-`#[ignore]` tests:
  - `test_ruyi_throw_aborts_when_unwind_returns`
  - `test_ruyi_end_catch_no_panic_without_active_catch`
  - `test_function_exception_table_multiple_entries`
  - `test_nested_try_catch_propagates_to_outer`

##### Scenario: try_catch_invoke tests

- **WHEN** running `cargo test -p ruyic --test try_catch_invoke`
- **THEN** all tests SHALL pass (previously some were blocked by "Complex new expressions" limitation, which Change B resolves)

##### Scenario: No regression in codegen tests

- **WHEN** running `cargo test -p ruyic --test codegen`
- **THEN** all previously-passing tests SHALL continue to pass
- **AND** no test SHALL exhibit new failures due to the unwinder changes
