# Proposal: fix-codegen-string-dispatch

## Why

Code like `basename("/path").length` or any Ruyi expression that calls a method on the result of a function returning `string` (or `int`/`float`/`bool`/`Array`) hits either a runtime panic `"Cannot call method on call result type: <type>"` or produces an LLVM link error from a missing FFI symbol.

The codegen path in `crates/ruyic/src/codegen/expr.rs:2403-2427` recognizes `Expr::Member` whose receiver is itself an `Expr::Call`, but:

1. Fails to extract a class_name when `result.ty == Type::String` / `Type::Int` / `Type::Float` / `Type::Bool` (these don't match `Type::Named`) — emitting the panic.
2. Even when the class_name is extractable, looks up `<Class>_<method>` (e.g. `String_length`) which **does not exist** in `crates/ruyic/src/codegen/builtins_table.rs::BUILTINS`. The runtime exposes `__string_length`, `__string_substring`, `__string_indexOf` etc.

Reproduction (`/tmp/v_path.ry`):
```ry
import { basename } from "fs";
println(basename("/home/user/file.txt").length);
```
Result: `codegen error: Cannot call method on call result type: String` (compile-time error).

## Root Cause (with code cite)

`compile_expr` in `crates/ruyic/src/codegen/expr.rs:2403-2427` (the `Expr::Member { object, property, ..}` arm):

```rust
Expr::Call {
    callee: inner_callee,
    args: inner_args,
} => {
    let result = compile_call(ctx, inner_callee, inner_args)?;
    let class_name = match &result.ty {
        Type::Named(n, _) => n.clone(),
        _ => return Err(format!(
            "Cannot call method on call result type: {:?}",
            result.ty
        )),
    };
    let ptr = match result.value {
        BasicValueEnum::PointerValue(p) => p,
        _ => return Err("Call result is not a pointer value..."),
    };
    (Some(ptr), class_name)
}
```

Two problems:

**A.** The match only handles `Type::Named(_)` — primitives `Type::String`, `Type::Int`, `Type::Float`, `Type::Bool` are not named types, so they always error out.

**B.** Even if `Type::String` were mapped to `"String"`, the subsequent code (line 2430-2470) looks up the FFI symbol as `format!("{}_{}", class_name, method_name)` which yields `String_length`. The actual runtime export is `__string_length` (one leading underscore, and uses snake_case not camelCase).

## What Changes

### File 1 — `crates/ruyic/src/codegen/expr.rs`

#### Edit A: extend the type-to-class match (line 2408)

```rust
let class_name = match &result.ty {
    Type::Named(n, _) => n.clone(),
    Type::String => "String".to_string(),
    Type::Int => "Int".to_string(),
    Type::Float => "Float".to_string(),
    Type::Bool => "Bool".to_string(),
    Type::Array(_) => "Array".to_string(),
    _ => return Err(format!(
        "Cannot call method on call result type: {:?}",
        result.ty
    )),
};
```

#### Edit B: introduce a method-symbol resolution table

Right after Edit A, add a mapping function:
```rust
fn resolve_method_symbol(class_name: &str, method_name: &str) -> Option<&'static str> {
    // String methods → __string_<method> convention
    if class_name == "String" {
        return Some(match method_name {
            "length" => "__string_length",
            "substring" => "__string_substring",
            "indexOf" => "__string_indexOf",
            "charAt" => "__string_char_at",
            "startsWith" => "__string_starts_with",
            "endsWith" => "__string_ends_with",
            "trim" => "__string_trim",
            "toLowerCase" => "__string_to_lower",
            "toUpperCase" => "__string_to_upper",
            "concat" => "__string_concat",
            // ... continue mapping the full stdlib/string.ry surface
            _ => return None,
        });
    }
    // Int → ruyi_int_<method>
    if class_name == "Int" {
        return Some(match method_name {
            "toString" => "ruyi_int_to_string",
            "abs" => "ruyi_int_abs",
            // ...
            _ => return None,
        });
    }
    // Float, Bool, Array follow the same pattern
    None
}
```

Place this in `crates/ruyic/src/codegen/expr.rs` (same file as `compile_call`), and have the `Expr::Member` arm at line 2430 call it:

```rust
let func_name = match resolve_method_symbol(class_name, method_name) {
    Some(n) => n.to_string(),
    None => format!("{}_{}", class_name, method_name),
};
```

(Keep the existing fallback `format!("{}_{}", class_name, method_name)` for class instances and trait impls — those use the legacy dispatch.)

### File 2 — `crates/ruyic/src/codegen/builtins_table.rs`

Add to the `BUILTINS` table (currently 142 entries per `crates/ruyic/src/codegen/builtins_table.rs:51-840`) every `__string_*` / `__int_*` / `__float_*` / `__bool_*` / `__array_*` FFI symbol that `resolve_method_symbol` references, IF NOT ALREADY PRESENT. Most of `__string_*` already exist (created in v0.5.8 stdlib expansion + v0.5.9 P2 expansion). The new content is the formal registry entries, no new runtime functions.

### File 3 — `tests/integration/codegen_primitive_method.rs` (NEW)

Regression tests:
- `string_length_after_call`: `let n: int = "hello".length;` (compile + run; assert n == 5).
- `int_tostring_after_call`: `(1 + 2).toString()` (compile + run; assert == "3").
- `float_tostring_after_call`: `(1.5 + 2.5).toString()` (compile + run).
- `array_length_after_call`: `let arr: Array<int> = [1,2,3]; let n = arr.length;` (assert n == 3).
- `string_concat_via_method`: `"hello".concat(" ").concat("world")` (assert == "hello world").
- `nested_call_method_chain`: `"hello".toUpperCase().substring(0, 3)` (assert == "HEL").

## Acceptance Criteria

1. `make check` / `make build-release` pass.
2. `/tmp/v_path.ry` (existing reproducer): `println(basename("/home/user/file.txt").length)` compiles and prints `11` (or whatever `<basename>.length` returns at runtime).
3. The 6 regression tests in `tests/integration/codegen_primitive_method.rs` pass.
4. `builtins_count_is_142` test continues passing (no count change unless new FFI symbols added).
5. **No new symbols exported from the runtime** — only consumed side; existing exports are formalized in `BUILTINS` table.

## Scope (in)

- `crates/ruyic/src/codegen/expr.rs` — Edits A and B (table-driven dispatch)
- `crates/ruyic/src/codegen/builtins_table.rs` — table completeness (no new functions)
- `crates/ruyic/tests/integration/codegen_primitive_method.rs` — NEW

## Scope (out / Scope Fence)

- ❌ Trait method dispatch (e.g. `(impl Printable).format()`) — already works at the codegen layer once `result.ty` resolves to a `Type::Named`. Different bug; defer.
- ❌ User-defined class instance methods with mismatched symbol names — covered by B-1.
- ❌ Removing stdlib workaround (`__string_length` FFI calls inside stdlib/string.ry) — done as follow-up `remove-stdlib-workarounds` after B-1 + B-2 + B-4 all land.

## Impact

| Dimension | Impact |
|-----------|--------|
| Compiler binary | unchanged |
| Compile time | +1 match per method call → negligible |
| e2e correctness | resolves `Cannot call method on call result type: String` for ALL primitive types |
| `cargo test` count | +6 integration tests |
| ABI | **no new exports** |

## Capabilities (CLOSED)

- `primitive-method-on-call-result`: any method on `string`/`int`/`float`/`bool`/`Array` result of a function compiles
- `string-method-table-driven-dispatch`: `String.length` ↔ `__string_length` mapping exhaustive
- `user-code-matches-stdlib-expectation`: stdlib can drop workarounds in follow-up
