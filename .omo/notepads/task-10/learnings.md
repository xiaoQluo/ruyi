# Task 10 Learnings

## Parser Limitations
- The parser doesn't support: `self` keyword in trait methods, `type` alias syntax, `throw` with function call args, `for(init;cond;update)` with let declarations, array literals with commas (sometimes), object type annotations `{ x: float }`, generic type annotations `Array<int>`, `void` type annotation
- The parser DOES support: most expression types, if/while/for-of/for-in, match, try/catch, class/trait declarations, function declarations, arrow functions, optional chaining, nullish coalescing

## Type System Design
- Gradual typing with `dyn` as the escape hatch is the right approach per spec Sections 8-11
- Bidirectional inference (synthesize + check) is the standard approach for gradual type systems
- Type narrowing for null safety is essential per spec Section 9.5
- Structural subtyping for objects: `{ more fields } <: { fewer fields }` (not the other way around)
- Function subtyping: contravariant in params, covariant in return

## Testing Strategy
- Tests that reference undeclared variables will produce type errors (correct behavior)
- Tests that depend on parser features not yet implemented should be marked #[ignore]
- The `check_program` helper should handle parse errors gracefully
- Use `assert_no_errors(&result)` for better error messages on failure
