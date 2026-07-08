# Spec: Codegen Declaration Skip for Macro & TypeAlias

## MODIFIED Requirements

### REQ-CG-001: Macro declaration SHALL NOT produce LLVM IR

The Ruyi compiler's codegen phase MUST skip macro declarations without emitting any LLVM IR, because macro expansion is performed by the macro_expand phase prior to type checking.

#### Scenario: Simple macro declaration compiles without error
- **WHEN** the source contains `macro foo { () => { ... } }`
- **THEN** codegen returns `Ok(())` and produces no IR for that declaration

#### Scenario: Macro followed by use of expansion compiles
- **WHEN** the source contains a macro declaration followed by a call to a macro call site
- **THEN** the entire source compiles and the expanded code executes correctly

### REQ-CG-002: Type alias declaration SHALL NOT produce LLVM IR

The Ruyi compiler's codegen phase MUST skip type alias declarations without emitting any LLVM IR, because type aliases are resolved at type-check time.

#### Scenario: Simple type alias compiles without error
- **WHEN** the source contains `type Name = string;`
- **THEN** codegen returns `Ok(())` and produces no IR for that declaration

#### Scenario: Type alias used as variable annotation compiles
- **WHEN** the source contains `type Name = string;` followed by `let x: Name = "hi";`
- **THEN** the entire source compiles and `x` holds a string value
