# Spec: Trait Constraint Check Actual Validation

## MODIFIED Requirements

### REQ-TRAIT-001: `check_bounds` MUST verify impl exists

`generics.rs::check_bounds()` MUST actually verify that for each generic type parameter with a trait bound, an `impl Trait for Type` exists in scope. The current implementation always returns `true` and MUST be replaced with a real implementation that walks the impl table.

#### Scenario: Generic with existing impl compiles
- **WHEN** source contains `fn print_it<T: Printable>(x: T) { ... }` and `impl Printable for int { ... }` is in scope
- **THEN** compilation succeeds

#### Scenario: Generic without impl fails compilation
- **WHEN** source contains `fn print_it<T: Printable>(x: T) { ... }` with NO impl of `Printable` for the concrete type used
- **THEN** compilation fails with "trait Printable not implemented for type X" error

#### Scenario: Multiple trait bounds all checked
- **WHEN** source contains `fn foo<T: Printable + Comparable>(x: T) { ... }` and only `impl Printable for int` exists
- **THEN** compilation fails with "trait Comparable not implemented for type int"

### REQ-TRAIT-002: At least 5 `#[ignore]` typechecker tests MUST pass

The 32 `#[ignore]` typechecker tests in `crates/ruyic/tests/typechecker.rs` MUST have at least 5 un-ignored and passing after this spec is implemented.

#### Scenario: 5+ typechecker tests un-ignored and pass
- **WHEN** `cargo test -p ruyic --test typechecker` is executed after implementation
- **THEN** at least 5 previously-`#[ignore]` tests are now `#[test]` and pass

### REQ-TRAIT-003: `impl Trait for Type` standalone block MUST be supported

The compiler MUST support standalone `impl Printable for int { ... }` blocks outside of class definitions (currently only in-class impl is supported).

#### Scenario: Standalone impl block compiles
- **WHEN** source contains `impl Printable for int { fn format(self): string { return "int"; } }` outside any class
- **THEN** compilation succeeds and the impl is registered in the impl table