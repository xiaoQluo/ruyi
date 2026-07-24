# Spec: codegen-unsupported-paths

## ADDED Requirements

### R1: Anonymous Function Codegen

The codegen SHALL support compiling anonymous function expressions (e.g., `let f = fn(x) { return x + 1; };`)
to executable LLVM IR.

##### Scenario: Simple anonymous function

- **WHEN** source contains `let double = fn(x: int): int { return x * 2; }; print(double(5));`
- **THEN** the codegen SHALL produce a callable function
- **AND** execution SHALL output `10`

##### Scenario: Anonymous function as argument

- **WHEN** source contains a higher-order function receiving an anonymous function
- **THEN** the codegen SHALL produce correct LLVM IR without "not yet supported" error

---

### R2: Async Arrow Function Codegen

The codegen SHALL support compiling async arrow function expressions.

##### Scenario: Async arrow function

- **WHEN** source contains `let f = async (x: int): Future<int> => x * 2;`
- **THEN** the codegen SHALL produce a compilable async function
- **AND** no "not yet supported" error SHALL be emitted

---

### R3: Nested Member Access Codegen

The codegen SHALL support compiling nested member access expressions
(e.g., `obj.prop.method()`, `a.b.c`).

##### Scenario: Chained field access

- **WHEN** source contains `let name = user.profile.name;`
- **THEN** the codegen SHALL resolve the full member chain and generate correct LLVM GEP + load

##### Scenario: Method call on field

- **WHEN** source contains `obj.calculator.add(1, 2);`
- **THEN** the codegen SHALL resolve `.calculator` as field access then call `.add()`
- **AND** no "nested member access not yet supported" error SHALL be emitted

---

### R4: Indirect Call Codegen

The codegen SHALL support calling functions through variables or expressions
(e.g., `let f = someFunc; f(args);`).

##### Scenario: Function variable call

- **WHEN** source stores a function reference in a variable and calls it
- **THEN** the codegen SHALL resolve the function pointer and generate an indirect call

##### Scenario: Function pointer from field

- **WHEN** source calls a function stored as an object field
- **THEN** the codegen SHALL load the function pointer and perform an indirect call

---

### R5: Spread Arguments Codegen

The codegen SHALL support spread arguments in function calls, constructor calls,
and super constructor calls (e.g., `fn(...args)`, `new Foo(...args)`, `super(...args)`).

##### Scenario: Spread in function call

- **WHEN** source contains `fn combine(a, b, c) { ... }; let arr = [1, 2, 3]; combine(arr[0], arr[1], arr[2]);`
- **THEN** the codegen SHALL compile without "spread arguments not yet supported" error

##### Scenario: Spread in constructor call

- **WHEN** source contains `new Foo(...args)` where `args` is an array
- **THEN** the codegen SHALL unpack arguments and call the constructor

##### Scenario: Spread in super constructor call

- **WHEN** a subclass constructor calls `super(...args)`
- **THEN** the codegen SHALL unpack arguments and call the parent constructor

---

### R6: Compound Assignment Codegen

The codegen SHALL support compound assignment operators: `+=`, `-=`, `*=`, `/=`, `%=`.

##### Scenario: Add-assign on variable

- **WHEN** source contains `let x = 5; x += 3; print(x);`
- **THEN** the codegen SHALL generate a load-add-store sequence
- **AND** execution SHALL output `8`

##### Scenario: Subtract-assign on field

- **WHEN** source contains `obj.count -= 1;`
- **THEN** the codegen SHALL load the field value, subtract, and store back

---

### R7: Complex Assignment Codegen

The codegen SHALL support assignments to targets beyond simple identifiers and fields,
including array index assignments.

##### Scenario: Array index assignment

- **WHEN** source contains `arr[0] = 42;`
- **THEN** the codegen SHALL generate a store to the array element
- **AND** no "complex assignments not yet supported" error SHALL be emitted

---

### R8: Complex New Expression Codegen

The codegen SHALL support `new` expressions where the callee is not a simple identifier
(e.g., `new getClass()(args)` or `new (expr)(args)`).

##### Scenario: New with expression callee

- **WHEN** source uses `new` with a non-identifier class expression
- **THEN** the codegen SHALL evaluate the expression to obtain the class type and allocate accordingly
- **AND** no "complex new expressions not yet supported" error SHALL be emitted

##### Scenario: Throw new Error with expression

- **WHEN** source contains `throw Error.new("message");`
- **THEN** the codegen SHALL compile correctly (this pattern was previously blocked)

---

### R9: Complex Pattern Binding Codegen

The codegen SHALL support pattern bindings beyond simple `Identifier` in `let` and `const`
declarations (e.g., destructuring).

##### Scenario: Array destructuring

- **WHEN** source contains `let [a, b] = [1, 2];`
- **THEN** the codegen SHALL bind `a` to the first element and `b` to the second
- **AND** no "complex patterns not yet supported" error SHALL be emitted

##### Scenario: Object destructuring

- **WHEN** source contains `let { x, y } = point;`
- **THEN** the codegen SHALL extract `x` and `y` fields from `point`

---

## MODIFIED Requirements

### R10: Error Reporting for Unsupported Paths

For any codegen path that remains truly unimplementable (beyond the 12 paths addressed here),
the error message SHALL include file location and a descriptive reason, rather than a generic
"not yet supported" string.

##### Scenario: Truly unsupported codegen pattern

- **WHEN** the codegen encounters a pattern that cannot be implemented with current infrastructure
- **THEN** the error SHALL include the source file location (line:column)
- **AND** the error SHALL describe which language feature is unsupported
- **AND** the error SHALL use a consistent format: `"<feature>: not yet supported at <location>"`

---

## REMOVED Requirements

None. All 12 existing error paths are upgraded from "not yet supported" errors to functional
implementations. No behavior is removed.
