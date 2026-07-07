# Wire Function Annotations - Learnings

## Pattern for adding annotations to declarations
- Follow the `Class` pattern: add `annotations: Vec<String>` to AST variant, call `parse_annotations()` at start of parser function
- `parse_fn_declaration()`: annotations parsed BEFORE `async` keyword, matching the Class behavior where annotations come before `class`

## Routing peek-ahead for @annotated declarations
- `parse_declaration()` must disambiguate `@annot fn` vs `@annot class` by peeking ahead past `@Ident` pairs
- Peek offset = number of `@Ident` pairs * 2; check if next token is `Fn`/`Async` or `Class`
- Both `parse_fn_declaration()` and `parse_class_declaration()` start with `parse_annotations()` which consumes the `@` tokens, so the peek doesn't need to consume anything

## Files modified
1. `crates/ruyic/src/parser/ast.rs` - Added `annotations: Vec<String>` to `Declaration::Function` and `ExportDecl::DefaultFunction`
2. `crates/ruyic/src/parser/parser.rs` - Wired `parse_annotations()` in `parse_fn_declaration()`, updated routing in `parse_declaration()` and `parse_export()`
3. `crates/ruyic/src/typechecker/inference.rs` - Added `..` to destructuring
4. `crates/ruyic/src/codegen/expr.rs` - Added `annotations: vec![]` to constructed `Declaration::Function`

## Pre-existing test failures (11 tests)
All 11 failures exist on main before changes - not caused by this PR. They relate to:
- `Builtin("int")` vs `Identifier("int")` type annotation mismatch
- Trailing semicolons on `async fn` declarations
- `let_multiple` and `expr_new_with_args` parse validation
