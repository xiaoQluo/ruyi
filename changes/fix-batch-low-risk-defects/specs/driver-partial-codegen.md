# Spec: driver-partial-codegen

## MODIFIED Requirements

### R1: allow_partial_codegen Scope Control

The `allow_partial_codegen` flag in `CodeGenerator` SHALL be set to `true` only when
compiling standard library modules. For user code compilations, it SHALL be set to `false`.

##### Scenario: User code with unsupported pattern

- **WHEN** a user `.ry` file contains a codegen pattern that cannot be compiled
- **THEN** the compiler SHALL report a codegen error with file location and diagnostic message
- **AND** compilation SHALL fail with a non-zero exit code

##### Scenario: Stdlib compilation with unsupported pattern

- **WHEN** the stdlib module `.ry` file contains a codegen pattern that cannot be compiled
- **THEN** the compiler SHALL silently skip the unsupported expression and continue compilation
- **AND** compilation SHALL succeed (provided no other errors exist)

##### Scenario: Mixed user + stdlib compilation (default mode)

- **WHEN** a user `.ry` file is compiled (stdlib auto-loaded)
- **THEN** codegen errors in user code SHALL be reported as hard errors
- **AND** codegen errors in stdlib code SHALL be silently skipped

### R2: allow_partial_codegen Backward Compatibility

The change SHALL NOT cause regressions in any existing compilation test cases.
All previously-passing tests that compile `.ry` files with the compiler SHALL continue to pass.

##### Scenario: Existing passing integration tests

- **WHEN** running `cargo test -p ruyic --test codegen`
- **THEN** all previously-passing tests SHALL continue to pass
- **AND** no test SHALL exhibit new failures due to this change
