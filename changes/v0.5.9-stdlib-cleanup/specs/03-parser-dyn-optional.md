# Spec 03: parser-dyn-optional — Add 3 grammar productions to `parser/parser.rs`

## Overview

`v0.5.8-stdlib-core` introduced `stdlib/json.ry` and `stdlib/random.ry`, both of which use parser features that fail at parse time:
- `stdlib/json.ry:22-23`: `fn parse(s: string): dyn { return __json_parse(s); }` — `dyn` as return type
- `stdlib/json.ry:32`: `fn stringify(v: dyn): string { ... }` — `dyn` as parameter type
- `stdlib/random.ry`: `fn parseInt(s: int? = 0): int { ... }` — `?:` optional parameter with default value

`cargo build --release` succeeds; `cargo test -p ruyi_runtime --lib` succeeds; `ruyic --check stdlib/math.ry` succeeds. But `ruyic --check stdlib/json.ry` and `ruyic --check stdlib/random.ry` fail with "parse error: Expected identifier but found 'keyword 'return''".

This spec adds the 3 missing grammar productions.

## Requirements

### REQ-1: `dyn` as return type
**SHALL** modify `crates/ruyic/src/parser/parser.rs` so that the production `fn f(): dyn { ... }` parses to a function with return type `Type::Dynamic`.

**Current behavior**: `parser.rs:1904` (line reference from v0.5.8 inspection) handles `Token::Dyn` only in the context of `dyn Trait` (i.e., expecting a follow-up `Token::Ident` for the trait name). When `Token::Dyn` is followed by `{` (function body start), the parser fails.

**Required change**: in the `parse_return_type` (or equivalent) function, when `Token::Dyn` is followed by `{`, return `Type::Dynamic` directly (no follow-up trait expected).

### REQ-2: `dyn` as parameter type
**SHALL** modify `parse_param_type` so that `dyn` standalone (in `(x: dyn, ...)`) parses to `Type::Dynamic`.

**Current behavior**: same `Token::Dyn` handler, but in parameter position. Same fix.

### REQ-3: `?:` optional parameter syntax
**SHALL** modify `parse_param_list` so that `(s: int? = 0)` parses to a parameter with:
- `name = "s"`
- `type_annotation = Some(Type::Int)` (with optionality bit)
- `default_value = Some(...)` (the expression after `=`)

**Current behavior**: the parser fails when seeing `?` in a parameter type position.

**Required change**: in `parse_param_type`, when seeing `<type>?`, mark the type as optional (e.g., `Type::Optional(Box::new(Type::Int))` or a separate `Optional` flag on the parameter AST node). Then in `parse_param_list`, after parsing the parameter, optionally consume `= <default_expr>`.

**Typechecker propagation**: `Type::Optional(T)` should be assignable from `T` and from `null`. Method calls on `Type::Optional(T)` should be `T`'s methods. `default_value` is bound at the function's entry: if the caller doesn't pass the argument, `default_value` is evaluated and assigned to the parameter.

**Codegen propagation**: when generating a call to a function with optional parameters, the codegen generates code that:
- If the caller passes the argument: use the argument
- If the caller doesn't pass: evaluate the default value expression and use it

## Scenarios

### SCEN-1: `dyn` return type parses
**WHEN** parsing `fn f(): dyn { return 1; }`
**THEN** produces a `Function { params: [], return_type: Type::Dynamic, body: [Return(1)] }`

**Acceptance**:
```bash
echo 'fn f(): dyn { return 1; }
fn main() { f(); }' > /tmp/test_dyn.ry
./target/release/ruyic --check /tmp/test_dyn.ry
# → "Type checking passed."
```

### SCEN-2: `dyn` parameter type parses
**WHEN** parsing `fn f(x: dyn): dyn { return x; }`
**THEN** produces a function with parameter `x: Type::Dynamic`

**Acceptance**: similar to SCEN-1.

### SCEN-3: `?:` optional parameter parses
**WHEN** parsing `fn f(s: int? = 0): int { return s; }`
**THEN** produces a function with optional parameter `s` defaulting to `0`

**Acceptance**:
```bash
echo 'fn f(s: int? = 0): int { return s; }
fn main() { let _ = f(); }' > /tmp/test_opt.ry
./target/release/ruyic --check /tmp/test_opt.ry
# → "Type checking passed."
```

### SCEN-4: `stdlib/json.ry` `--check` end-to-end
**WHEN** `ruyic --check stdlib/json.ry` is invoked
**THEN** output is "Type checking passed." (Full e2e key acceptance for v0.5.9)

**Acceptance**: command exits 0 with "Type checking passed." in stdout

### SCEN-5: `stdlib/random.ry` `--check` end-to-end
**WHEN** `ruyic --check stdlib/random.ry` is invoked
**THEN** output is "Type checking passed." (same-family bug)

**Acceptance**: command exits 0

### SCEN-6: No regression on existing 33 examples
**WHEN** `bash examples/run_examples.sh` runs after parser changes
**THEN** 33/33 still pass

**Acceptance**: 33/33 exit 0

## Out of Scope

- Other parser bugs (only the 3 in scope fix; any other reported issue is a separate change)
- `dyn` as a generic type argument (e.g., `Array<dyn>`) — not in current stdlib usage
- Default value type inference improvements (e.g., allowing the default to omit the type annotation)
- Making `?:` work on struct fields (only function parameters are in scope)
- Optional parameter rest-spreading (e.g., `f(...args: int[])`) — separate feature

## Risks

| ID | Risk | Mitigation |
|----|------|------------|
| R2-1 | Parser change breaks AST → IR pipeline for existing programs | 33-example test suite is the oracle; any regression is git-revert of T3 commit |
| R2-2 | `Type::Optional` introduces a new Type variant that confuses existing typecheck code | Audit all `Type::X` match arms in `inference.rs`; add `Optional` arms where missing |
| R2-3 | Default value evaluation has wrong semantics (e.g., eager vs lazy) | Document: default is evaluated **once per call** if the argument is missing; no capture issues |
| R2-4 | Optional parameter codegen generates wrong number of arguments | Mirror the pattern used for `rest` parameters; smoke-test with 33-example suite |
