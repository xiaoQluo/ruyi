# Spec: BigInt Match Routing

## ADDED Requirements

### REQ-BM-003: BigInt literal match MUST compare via runtime equality function

When a `match` expression has a BigInt scrutinee and an arm uses a BigInt literal pattern, the codegen MUST compare the scrutinee against the literal by calling a runtime equality function (e.g. `ruyi_bigint_eq`), because BigInt values cannot be compared via LLVM integer comparison alone.

The runtime function MUST be added to the `ruyi_runtime` crate's builtins (where `ruyi_bigint_from_str` already lives), and it MUST compare two BigInt values by value and return an `i8` (0 = false, non-zero = true) or equivalent.

#### Scenario: BigInt literal match compiles and dispatches correctly
- **WHEN** the source contains `match (n: bigint) { 42n => "forty-two", _ => "other" }`
- **THEN** the program compiles, the runtime equality function is called for the literal arm, and the correct arm executes based on the value of `n`

#### Scenario: Wildcard fallback still works after literal arm
- **WHEN** the source contains `match (n: bigint) { 42n => "forty-two", _ => "other" }` and `n` is `100n`
- **THEN** the program returns `"other"` via the wildcard arm

## MODIFIED Requirements

### REQ-BM-001: BigInt scrutinee in match expression MUST NOT route to integer-match codegen

The Ruyi compiler's codegen phase MUST NOT route `Type::BigInt` scrutinee through the integer-match codegen path (`compile_int_match`), because BigInt's LLVM representation is a pointer (`i8*`) and not a 64-bit integer.

The BigInt scrutinee MUST be routed to the generic match codegen path (`compile_generic_match`), which handles wildcard, identifier, and other non-literal patterns correctly.

#### Scenario: Wildcard-only match on BigInt compiles and runs
- **WHEN** the source contains `fn f(n: bigint): string { match (n) { _ => { return "x"; } } }`
- **THEN** the program compiles, the function returns `"x"` for any BigInt input

#### Scenario: Integer scrutinee match continues to work (regression guard)
- **WHEN** the source contains `match (x: int) { 1 => A, _ => B }`
- **THEN** the program compiles and the integer switch path is used (not the generic fallback)

### REQ-BM-002: Int match codegen path MUST remain unchanged for non-BigInt integers

The existing `compile_int_match` and its sub-functions (`compile_int_match_switch`, `compile_int_match_sequential`) MUST continue to handle `Type::Int` exactly as before, with no behavior changes.
