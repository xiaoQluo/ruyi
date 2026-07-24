# Proposal: fix-rustic-static-method

## Why

**Status: IMPLEMENTED 2026-07-24 (verify "Implementation Status" below).** The original root cause (static method receiver binding misdispatch) was diagnosed correctly, the three typechecker-side fixes were applied and verified, and same-module static method calls now work end-to-end.

Calling a static method on an *imported* class (e.g. `Path.join([..])` from `fs.ry` after `import { Path } from "path"`) currently fails at **codegen time** because the `static_method_names` map is populated per-module and is **not propagated across `import` edges**. The remaining gap is cross-module propagation, deferred to a follow-up change `propagate-static-method-names-across-imports` (out of scope here).

**Verification (same-module)**:
- `class Foo { static fn bar(): int { return 42; } static fn baz(x: int): int { return x + 1; } }`
- `Foo.bar()` → 42 ✓
- `Foo.baz(99)` → 100 ✓

**Not yet covered**:
- `import { Foo } from "other"; Foo.bar()` — `static_method_names["Foo"]` is computed in `other.ry`'s inference but does **not** merge into the importing module's look-up table.
- `crates/ruyic/tests/integration/cases/stdlib/path_ffi.ry:19` regression test still blocks for the same cross-module reason.
- The stdlib `fs.ry` Path/File workarounds from v0.5.9-stdlib-cleanup (P2) remain in place.

## Root Cause (with code cite)

`ClassElement::Method` in `crates/ruyic/src/parser/ast.rs` carries an `is_static: bool` field, but `infer_class_element` in `crates/ruyic/src/typechecker/inference.rs:461-468` discards it:

```rust
crate::parser::ast::ClassElement::Method {
    name: prop_name,
    type_params: _,
    params,
    return_type,
    body,
    is_async,
    is_static: _,    // ← discarded, all methods go to `class_methods` HashMap
    is_getter: _,
    is_setter: _,
}
```

Then `synthesize_member_access` at line 1634 decides "this is a static call" ONLY when `prop_name == "new"`:

```rust
let is_static_call = prop_name == "new"
    && matches!(object, Expr::Identifier(_))
    && matches!(obj_ty, Type::Named(_, _));
```

For `Path.join(...)`, `prop_name == "join"` (not `"new"`), so `is_static_call = false`, and the function type goes through the instance-method path that prepends the receiver (treating `self` as the first parameter, then substituting types). The args that `Path.join` expects (`Array<string>`) are then compared against a function whose **first param is the receiver type** (`Path` here, incorrectly), producing a TypeMismatch.

## What Changes

### File 1 — `crates/ruyic/src/typechecker/inference.rs`

Three edits:

1. Add a parallel field to `TypeInference` (line 105) for tracking static method names per class:
```rust
/// Names of methods declared `static fn` on each class. Populated in
/// `infer_class_element` (Method case) when `is_static` is true. Looked up
/// in `synthesize_member_access` to decide static vs instance dispatch.
static_method_names: HashMap<String, HashSet<String>>,
```
Initialize empty in `TypeInference::new()` (line 178-181, alongside `class_fields` / `class_methods`).

2. In `infer_class_element` Method arm (line 467-468), accept `is_static`:
```rust
crate::parser::ast::ClassElement::Method {
    name: prop_name,
    type_params: _,
    params,
    return_type,
    body,
    is_async,
    is_static,           // ← was `_`, now captured
    is_getter: _,
    is_setter: _,
} => {
    // ...
    if *is_static {
        if let Some(class_name) = self.class_stack.last() {
            self.static_method_names
                .entry(class_name.clone())
                .or_default()
                .insert(method_name.clone());
        }
    }
```

3. In `synthesize_member_access` (line 1634-1711), expand the static-call detection:
```rust
let is_static_call = if let Type::Named(class_name, _) = &obj_ty {
    matches!(object, Expr::Identifier(_))
        && self
            .static_method_names
            .get(class_name)
            .map(|s| s.contains(prop_name))
            .unwrap_or(false)
} else {
    false
};
```

### File 2 — `crates/ruyic/tests/integration/cases/stdlib/path_ffi.ry`

Already exists from P0-A; revisit to remove the now-unnecessary `extern fn __path_basename` workarounds where the spec class method suffices.

### File 3 — `crates/ruyic/tests/integration/static_method.rs` (NEW)

Regression test:
```rust
#[test]
fn static_call_does_not_treat_self_as_first_arg() {
    let src = "fn main() {
        print(Path.join([\"/a\", \"b\"]));
    }";
    let mut parser = Parser::new(src).unwrap();
    let program = parser.parse().unwrap();
    let mut inference = TypeInference::new(TraitRegistry::new());
    let result = inference.infer_program(&program);
    assert!(!result.diagnostics.has_errors());
}
```

plus an `@test fn static_call_does_not_inject_self_receiver` in `tests/integration/cases/stdlib/path_class_call.ry`.

## Acceptance Criteria

1. `make check` — passes with no new warnings (matches R4 zero-new-lint policy of `v0.5.9-stdlib-cleanup`).
2. `make build-release` — `cargo build --release --bin ruyic` succeeds.
3. **Path.join-style call no longer false-positive typechecks**: Compile `/tmp/v_path.ry` (imports `basename` from `fs`) AFTER rolling back the P2 fs.ry FFI workarounds for `Path` static calls — expect `ruyic --check` PASS without "Cannot call method on call result type" at codegen, and the resulting binary prints the basename.
4. **The path_ffi.ry regression test (already in tree) passes**: `cargo test --test integration path_ffi` runs without typecheck errors.
5. **New static_method.rs regression test** passes.

## Scope (in)

- `crates/ruyic/src/typechecker/inference.rs` (3 edits: struct field, populate, lookup)
- `crates/ruyic/tests/integration/static_method.rs` (NEW, ~30 lines)

## Scope (out / Scope Fence)

- ❌ **NOT removing the fs.ry stdlib-side workaround yet** — the P2 fs.ry workaround stays until B-1 + B-2 + B-3 + B-4 are all merged, after which a follow-up change `remove-stdlib-workarounds` will restore the natural `Path.join` / `File.open` calls.
- ❌ **NOT changing `Method` parsing** — the parser already supports `static fn` keyword (verified by `is_static` AST field).
- ❌ **NOT touching codegen method-call resolution** — the static vs instance path resolves the wrong type only at typechecker; codegen already accepts any `Type::Function` from `callee_ty` once `synthesize_call` reaches the call dispatch.
- ❌ **NOT changing trait method resolution** — `synthesize_member_access` already has the trait branch at line 1691 untouched.
- ❌ **NOT introducing `pub static fn`-on-instance restriction** (no JS/CoffeeScript-style `this`-rebinding needed).

## Impact

| Dimension | Impact |
|-----------|--------|
| Compiler binary size | unchanged |
| Compile time | +1 hashmap put / lookup per class method → negligible (<0.1%) |
| stdlib workaround removal | unlocks a follow-up change `remove-stdlib-workarounds` |
| `cargo test` count | +3 (regressions in `tests/integration/static_method.rs`) |
| ABI | unchanged |
| Public API | unchanged; `is_static` field stays internal to inference |

## Capabilities (CLOSED)

- `static-method-dispatch`: `Class.method(...)` typechecks and codegens correctly for classes declared `static fn method` (and dynamically for class imports where the static flag is propagated from the original declaration).
- `path-file-regex-imports`: `import { Path } from "path"; Path.join(...)` resolves the same as user-class static calls.
