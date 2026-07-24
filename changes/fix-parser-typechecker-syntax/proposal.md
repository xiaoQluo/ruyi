# Proposal: fix-parser-typechecker-syntax

## Why

Eleven specific syntax/typecheck constructs block end-to-end Ruyi programs even though `--check` passes through stdlib workarounds (see v0.5.9-stdlib-cleanup). All eleven were re-tested against `target/release/ruyic v0.5.9` on 2026-07-24; the failures are reproducible. This change fixes them so users can write idiomatic Ruyi code.

**Reproducer summary** (failures observed):

| # | Construct | Current error | Tested file |
|---|-----------|---------------|-------------|
| 1 | `int[]` function parameter type | `parse error: Expected ')' but found '['` | `/tmp/probe_b2/t1_array.ry` |
| 2 | `int[]` local variable type | `parse error: Expected ';' but found '['` | `/tmp/probe_b2/t1b_arrloc.ry` |
| 3 | `extern fn __path_basename(s: string): string;` | `parse error: Expected ';' but found 'fn'` | `/tmp/probe_b2/t2_extern.ry` |
| 4 | Nested generic `Future<Array<string>>` | `parse error: Expected '>' but found '>>'` | `/tmp/probe_b2/t3_nestedg.ry` |
| 5 | Lambda with block body `fn(a: int, b: int) -> int { ... }` | `parse error: Expected '{' but found '=>'` | `/tmp/probe_b2/t4_lambdablock.ry` |
| 6 | `new Class()` constructor call | `Type 'Box' is not callable` | `/tmp/probe_b2/t5_newclass.ry` |
| 7 | `if (cond) { return; }` in `void` fn | `Type mismatch: expected 'void', but found 'null'` | `/tmp/probe_b2/t6_earlyreturn.ry` |
| 8 | `if (!(cond)) { ... return; }` (workaround) | same: `expected 'void', but found 'null'` | `/tmp/probe_b2/t6b_invret.ry` |
| 9 | `export const FOO` then use `FOO` in same module | `Unknown variable: 'FOO'` | `/tmp/probe_b2/t7_exportconst.ry` |
| 10 | `async fn main(): Future<void> { print(...); }` | `expected Future<void>, but found Future<Future<void>>` | `/tmp/probe_b2/t8_futvoid.ry` |
| 11 | `s.length` (string method on FFI result) | codegen: `Cannot call method on call result type: String` | `/tmp/probe_b2/t9_stringmethod.ry` (overlaps B-4) |

## Root Causes (with code cites)

### 1, 2 — `parse_type` doesn't accept `[]` suffix
`parse_type` in `crates/ruyic/src/parser/parser.rs:1874` reads one type token (`Identifier`/`Generic`/`Function`/`Nullable`); no recursion into `[…]` array suffix.

### 3 — `extern fn` syntax
`parse_module_item` (line 209) at the top-level dispatch does not match `Token::Extern`; the keyword is recognized by the lexer (`Extern` token exists) but has no parser handler.

### 4 — nested `>` in generics
Generic args are delimited by `Less`/`Greater` tokens (line 1874 `parse_type`), but `>>` is tokenized as a single `ShiftRight` token (not split), so `Future<Array<string>>` after the inner `>` still has tokens `>` `>` left. `parse_type_args` walks the token list until the next `Greater`, but never cracks `>>` into two `>`.

### 5 — Lambda expression vs fn declaration ambiguity
Lambda expression `fn(a, b) -> int { ... }` starting at expression position is currently dispatched to `parse_fn_declaration` (line 461), which expects a name after `fn`; the lambda path is missing.

### 6 — `new Class()` not callable
The typechecker returns `Type::Class("Class")` for a class-as-value identifier, but `synthesize_call` (line 1049) match arm only handles `Type::Function` / `Type::Dynamic` / `Type::Error`. A method named `new` on a class is treated as instance method (covered by B-1); `new Class(args)` is parsed as a single call expr where callee is the keyword `new` (not handled).

### 7, 8 — `return;` in `void` fn typecheck
`infer_return_type` (line 1973) collects last statements — bare `return;` produces `Type::Null` (line 938), then `least_upper_bound` accumulates Null into the return type. For `void` fn the accumulated type must collapse to `Type::Void` not `Type::Null`.

### 9 — `export const` not in env
`infer_program` pass 1 (line 184-224) collects function declarations but not `export const`. So `FOO` from `export const FOO = 42;` is never declared, and the body references fail with `Unknown variable`.

### 10 — async void fn return shape
`infer_return_type` (line 1973) treats `print(...);` as a `Void` statement and the function's last expression (the `print(...)` result wrapped) becomes `Type::Future(Type::Void)`. The fn return is declared `Future<void>`, but the body's `last-typed-statement` produces `Future<Type::Void>` again, leading to `Future<Future<void>>`.

### 11 — covered by **B-4** (`fix-codegen-string-dispatch`)

## What Changes

### File 1 — `crates/ruyic/src/parser/parser.rs`

#### Edit A: array suffix in `parse_type`
After `parse_type` body (line 1874), if next token is `[`, consume `[ ]` and wrap in `TypeAnnotation::Array`:
```rust
let mut ty = self.parse_type()?;
while self.match_token(&Token::LBracket) {
    self.expect(Token::RBracket)?;
    ty = TypeAnnotation::Array(Box::new(ty));
}
Ok(ty)
```

#### Edit B: `extern fn` declaration
`parse_declaration` (line 389) extend match:
```rust
Some(Token::Extern) => self.parse_extern_decl(),
```

`parse_extern_decl` (new):
```rust
fn parse_extern_declaration(&mut self) -> Result<Declaration, ParseError> {
    self.expect(Token::Extern)?;
    self.expect(Token::Fn)?;
    let name = self.expect_ident()?;
    self.expect(Token::LParen)?;
    let params = self.parse_formal_params()?;
    self.expect(Token::RParen)?;
    let return_type = if self.check(&Token::Colon) {
        Some(self.parse_type_annotation()?)
    } else { None };
    self.expect(Token::SemiColon)?;
    Ok(Declaration::ExternFn { name, params, return_type })
}
```
Add `Declaration::ExternFn { name, params, return_type }` to ast.rs. AST already has the token; the typechecker already treats `__path_basename` as a builtin look-up via `resolve_builtin_name` (inference.rs:44), so the only new step is to forward-declare `__path_basename` in the env when `extern fn` is parsed.

#### Edit C: nested generic `>>` and `>>>`
In `parse_type_args` (around line 1867-1874):
- Accept `ShiftRight`/`ShiftRightAssign` ending tokens as terminator;
- Two-pass: collect greedily then split `>>` from the tail.

#### Edit D: lambda body
In `parse_expression` add an early branch:
```rust
if self.check(&Token::Fn) {
    return self.parse_lambda_with_block();
}
```
`parse_lambda_with_block` (new) handles `fn (params) [-> type] { body }` similar to `parse_fn_declaration` but returns `Expr::Lambda { params, return_type, body }` instead of `Declaration::Function`.

### File 2 — `crates/ruyic/src/parser/ast.rs`

Add:
```rust
Declaration::ExternFn {
    name: String,
    params: Vec<Param>,
    return_type: Option<TypeAnnotation>,
},
Expr::Lambda {
    params: Vec<Param>,
    return_type: Option<TypeAnnotation>,
    body: Vec<Statement>,
},
```

### File 3 — `crates/ruyic/src/typechecker/inference.rs`

#### Edit E: env seeding for export const
In `infer_program` pass 1 (line 184-224), add arm:
```rust
Declaration::ExternFn { name, .. } => {
    self.env.declare_let(name, Type::Dynamic);
}
ModuleItem::Export(ExportDecl::Declaration(Declaration::Const(bindings))) |
ModuleItem::Export(ExportDecl::Declaration(Declaration::Let(bindings))) => {
    for b in bindings {
        if let Pattern::Identifier(n) = &b.pattern {
            let ty = b.ty.as_ref().map(Type::from_annotation).unwrap_or(Type::Dynamic);
            self.env.declare_let(n, ty);
        }
    }
}
```

#### Edit F: `new Class()` call
Add a new arm to `synthesize` (line 1049):
```rust
Expr::New { class_name, args } => {
    // Resolve `class_name` to its constructor signature.
    let arg_types: Vec<Type> = args.iter().map(|a| self.synthesize(a)).collect();
    if let Some(constructor_ty) = self.tracker.constructor_for(class_name) {
        return Type::Named(class_name.clone(), vec![]);
    }
    // Fallback: infer as instance type
    Type::Named(class_name.clone(), vec![])
}
```

(Add `New` to `Expr` enum; replace `new Class(args)` parsing in `parse_primary_expression`.)

#### Edit G: `void` return consistency
In `infer_return_type` (line 1973) at the end:
```rust
let acc = /* existing accumulation */;
match self.return_type_stack.last() {
    Some(Type::Void) => Type::Void, // collapse any accumulated Null to Void
    _ => acc,
}
```

### File 4 — `crates/ruyic/tests/integration/syntax_gaps.rs` (NEW)

10 regression tests, one per reproducer (1-10; 11 lives in B-4).

## Acceptance Criteria

1. Each of the 10 reproducer .ry files (`/tmp/probe_b2/t1..t8.ry`) typechecks with `ruyic --check`, exit 0, no diagnostics.
2. `make check` / `make build-release` pass with no new warnings.
3. New `tests/integration/syntax_gaps.rs` passes 10 unit tests (one per fix).

## Scope (in)

- `crates/ruyic/src/parser/parser.rs` — Edits A-D
- `crates/ruyic/src/parser/ast.rs` — 2 new variants
- `crates/ruyic/src/typechecker/inference.rs` — Edits E-G
- `crates/ruyic/tests/integration/syntax_gaps.rs` — NEW (~150 LOC)

## Scope (out / Scope Fence)

- ❌ Static method receiver (B-1) — `new Class()` works but inside the constructor body, `self.foo` still hits B-1.
- ❌ String method dispatch (B-4) — repro item 11.
- ❌ `for-of` — **reproducer proved working** (output `6` from `[1,2,3].sum`); remove from "blocked list".
- ❌ Arg mutability, type aliases, async generator refactor (other parser bugs).
- ❌ Removing fs.ry stdlib workarounds — done as follow-up `remove-stdlib-workarounds` after this + B-1 + B-4 land.

## Impact

| Dimension | Impact |
|-----------|--------|
| Compiler binary | unchanged |
| Compile time | unchanged |
| stdlib `--check` | unchanged (still 24/24); stdlib e2e may improve |
| `cargo test` count | +10 unit tests + 1 parser regression |
| ABI | unchanged |
| User-visible language surface | +`extern fn`, +`Class[]` syntax, +block-body lambda, +`new X()`, +`if (cond) { return; }` works, +export const local use |

## Capabilities (CLOSED)

- `rfc-0014-extern-fn-syntax`: native `extern fn` declarations
- `rfc-0015-array-type-suffix`: `T[]` parameter and variable syntax
- `rfc-0016-lambda-blocks`: `fn(a, b) -> int { ... }` lambda body
- `rfc-0017-new-expression`: `new Class(args)` syntax
- `rfc-0018-export-const-resolution`: same-module `export const` resolves
- `rfc-0019-void-return-collapse`: early `return;` in `void` fn typechecks
