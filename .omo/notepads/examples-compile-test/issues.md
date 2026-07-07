# Example Compilation Issues

Date: 2026-05-04

## Summary
Compiled 6 example .ry files using `./target/release/ruyic` (v0.5.0).
**Passed: 1** (array.ry) | **Failed: 5** (control_flow.ry, functions.ry, variables_and_types.ry, try_catch.ry, error_handling.ry)

## Individual Failures

### 1. control_flow.ry — Parse Error (line 57, col 19)
- Error: `Expected identifier but found 'keyword 'from''`
- The `for...of` (or `for...in`) syntax may not be fully supported yet, or uses a different keyword.
- Line 57 likely has `for (let x from ...)` — the parser may expect `in` instead of `from`, or the for-range syntax differs from current parser.

### 2. functions.ry — Parse Error (line 101, col 23)
- Error: `Expected '=>' but found ':'`
- Likely using type annotation syntax (`fn foo(x: int)`) that conflicts with arrow function parsing, or the function syntax at line 101 uses a colon where `=>` is expected.
- Possibly a function return type annotation (`: int => ...`) that needs `=>`.

### 3. variables_and_types.ry — Codegen Error
- Error: `Unsupported binary operator: Power`
- The `**` operator (exponentiation/power) is parsed but not yet implemented in codegen.
- The file compiled through lexer/parser/typechecker OK, but LLVM codegen doesn't handle `Power` binary op.

### 4. try_catch.ry — Linker Error
- Error: `undefined symbol: ___gxx_personality_v0`
- The try/catch feature generates LLVM IR that references C++ exception handling personality function.
- The compiler compiled it to object code but linking fails — needs `-lstdc++` or similar flag.
- Known issue: the linker invocation doesn't include C++ exception libraries.

### 5. error_handling.ry — Parse Error (line 143, col 5)
- Error: `Unexpected token 'keyword 'catch''`
- The `catch` keyword at line 143 is not recognized by the parser in this context.
- Possibly the try/catch syntax in this file differs from what the compiler supports.

### 6. array.ry — SUCCESS ✅
- Compiled and linked without issues.
- Binary exists at `examples/target/array`.

---

## New Compilation Results (2026-05-04)

Compiled 7 example .ry files using `./target/release/ruyic` (v0.5.0).
**Passed: 2** (generics.ry, generics_simple.ry) | **Failed: 5** (type_system.ry, generics_comprehensive.ry, classes_and_objects.ry, traits.ry, pattern_matching.ry)

### 1. type_system.ry — Parse Error
- Error: `expected type at line 25, column 18`
- The parser encounters a type annotation it doesn't recognize at this position.

### 2. generics.ry — SUCCESS ✅
- Compiled successfully.
- Binary exists at `examples/target/generics`.

### 3. generics_simple.ry — SUCCESS ✅
- Compiled successfully.
- Binary exists at `examples/target/generics_simple`.

### 4. generics_comprehensive.ry — Parse Error
- Error: `Expected ';' but found ''!'' at line 50, column 22`
- Likely a postfix `!` operator (non-null assertion) not yet supported in this context.

### 5. classes_and_objects.ry — Codegen Error
- Error: `Unsupported binary operator: Power`
- Same issue as variables_and_types.ry — the `**` exponentiation operator lacks codegen support.

### 6. traits.ry — Type Mismatch
- Error: `Type mismatch: expected 'dyn Printable', but found 'string'` and `int`
- Trait object dispatch not fully working — the compiler doesn't properly coerce `string`/`int` to `dyn Printable`.

### 7. pattern_matching.ry — Parse Error
- Error: `Expected ';' but found ''}'' at line 176, column 26`
- Likely a syntax issue with match expression arms or guard syntax.
