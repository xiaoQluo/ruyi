# Spec 02: codegen-table-driven — Replace 60+ hand-written `fn declare_*` with a single `BUILTINS` table

## Overview

`crates/ruyic/src/codegen/builtins.rs` (1100 lines) contains 60+ hand-written `fn declare_*<'ctx>(context, module)` functions, each declaring exactly one LLVM `extern "C"` symbol. The `crates/ruyic/src/typechecker/inference.rs::resolve_builtin_name` function has a separate `match` block that names the same 35 (or so) FFI entries for typecheck. Adding any new FFI (e.g., the 20 new `__math_*` / `__time_*` / `__json_*` in v0.5.8) requires editing 3 places: `codegen/builtins.rs` (add a `fn declare_*` + call in `declare_builtins`), `typechecker/inference.rs` (add a `match` arm), and possibly `runtime/src/builtins.rs` (add a `pub use`).

This spec consolidates the 35 FFI entries into a single static `BUILTINS` table. Both codegen (LLVM `declare` instructions) and typechecker (type signatures) iterate the same table, ensuring the two layers cannot drift out of sync.

## Requirements

### REQ-1: `BUILTINS` table structure
**SHALL** create `crates/ruyic/src/codegen/builtins_table.rs` containing:

```rust
/// ABI-level signature for a builtin C FFI.
/// (Decoupled from Ruyi's `Type` to avoid circular dep with typechecker.)
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BuiltinSig {
    Void,    // () -> void
    Int,     // i64
    Float,   // f64
    String,  // *mut i8 (C string)
    Ptr,     // *mut i8 (opaque)
}

/// Static declaration of a builtin C FFI.
pub struct BuiltinDecl {
    pub name: &'static str,
    pub ret: BuiltinSig,
    pub params: &'static [BuiltinSig],
}

/// The 35 builtin FFI entries, deduplicated and canonical.
pub static BUILTINS: &[BuiltinDecl] = &[
    BuiltinDecl { name: "__builtin_array_create", ret: BuiltinSig::Ptr,   params: &[] },
    BuiltinDecl { name: "__builtin_array_get",    ret: BuiltinSig::Int,   params: &[BuiltinSig::Ptr, BuiltinSig::Int] },
    // ... (35 entries total) ...
    BuiltinDecl { name: "__string_replace_all",  ret: BuiltinSig::String, params: &[BuiltinSig::String, BuiltinSig::Int, BuiltinSig::String, BuiltinSig::Int, BuiltinSig::String, BuiltinSig::Int, BuiltinSig::String, BuiltinSig::Int] },
    BuiltinDecl { name: "__math_pi",             ret: BuiltinSig::Float, params: &[] },
    // ... (20 v0.5.8 entries) ...
];
```

### REQ-2: codegen dispatch
**SHALL** refactor `codegen/builtins.rs::declare_builtins` to iterate `BUILTINS`:

```rust
pub fn declare_builtins<'ctx>(context: &'ctx Context, module: &Module<'ctx>, gc_mode: GcMode) {
    // Special cases that don't fit the BUILTINS table (e.g., printf, gc_alloc, gc_collect)
    declare_printf(context, module);
    declare_alloc(context, module, gc_mode);
    declare_gc_collect(context, module);
    // ... 5-10 special-case declarations ...

    // Bulk: iterate the table
    for d in BUILTINS {
        let fn_type = sig_to_fn_type(context, d.ret, d.params);
        module.add_function(d.name, fn_type, None);
    }
}
```

Where `sig_to_fn_type` maps `BuiltinSig` to inkwell's `BasicTypeEnum` and builds the function type.

### REQ-3: typechecker sync
**SHALL** refactor `typechecker/inference.rs::resolve_builtin_name` to walk the same table:

```rust
fn resolve_builtin_name(name: &str) -> Option<Type> {
    // Special cases (RangeError, ArrayIterator)
    match name {
        "RangeError" => return Some(Type::Named("RangeError".to_string(), vec![])),
        "ArrayIterator" => return Some(Type::Named("ArrayIterator".to_string(), vec![])),
        _ => {}
    }
    // Walk BUILTINS
    for d in BUILTINS {
        if d.name == name {
            return Some(builtin_sig_to_type(d.ret, &d.params.iter().map(|p| builtin_sig_to_type(*p, &[])).collect::<Vec<_>>()));
        }
    }
    None
}
```

Where `builtin_sig_to_type` maps:
- `Void` → `Type::Void`
- `Int` → `Type::Int`
- `Float` → `Type::Float`
- `String` → `Type::String`
- `Ptr` → `Type::Dynamic`

### REQ-4: No API change
**SHALL** not modify any user-facing API. The `pub use crate::*_ffi::{...}` re-exports in `runtime/src/builtins.rs` remain unchanged. The codegen produces identical LLVM IR for every existing FFI.

## Scenarios

### SCEN-1: All 35 FFI declared
**WHEN** `codegen/builtins.rs::declare_builtins` is called
**THEN** exactly 35 (or however many in `BUILTINS`) function declarations are added to the module.

**Acceptance**:
```bash
make run-example EXAMPLE=math_demo
# Should print PI ≈ 3.14159, sqrt(16) = 4, etc.
```

### SCEN-2: Typechecker still recognizes all 35 FFI
**WHEN** any of the 35 FFI names is referenced in a Ruyi program
**THEN** `resolve_builtin_name` returns `Some(Type::Function { ... })` and the program typechecks.

**Acceptance**:
```bash
ruyic --check stdlib/collections.ry stdlib/string.ry stdlib/math.ry stdlib/time.ry stdlib/json.ry
# All → "Type checking passed."
```

### SCEN-3: No LLVM ABI change
**WHEN** the same FFI is called before and after the refactor
**THEN** the emitted LLVM IR has identical function signatures.

**Acceptance**:
```bash
git stash
make run-example EXAMPLE=math_demo
mv math_demo /tmp/math_before
git stash pop
make run-example EXAMPLE=math_demo
diff <(objdump -d /tmp/math_before | grep __math_abs) <(objdump -d math_demo | grep __math_abs)
# Empty diff
```

## Out of Scope

- Refactoring the `pub use` re-exports in `runtime/src/builtins.rs` (those live in the runtime crate, not codegen)
- Adding new FFI entries (this spec is purely a refactor; the 35 entries stay the same)
- Changing the LLVM ABI of any FFI (this spec is structure-only)
- `BUILTINS` table being `const`-friendly (Rust's `&'static` slice is sufficient; no `const fn` needed)

## Risks

- **R5-1**: A bug in `sig_to_fn_type` mapping produces wrong LLVM type signatures, causing runtime FFI miscalls. Mitigation: TDD with `cargo test -p ruyi_runtime --lib` (102+ tests) and the 33-example suite as oracle.
- **R5-2**: A bug in `builtin_sig_to_type` mapping produces wrong typecheck types, causing grad admission failures. Mitigation: same.
- **R5-3**: `BUILTINS` table ordering is wrong (e.g., a name typo in a string). Mitigation: each entry is a `&'static str`; any typo surfaces as `resolve_builtin_name` returning `None` and the user seeing "Unknown variable: `__math_pi`" at typecheck.

## Estimated Impact

- **Lines removed**: ~600 (60+ `fn declare_*` × 5 lines each)
- **Lines added**: ~250 (table population + 2 dispatch helpers + 2 typecheck helpers)
- **Net**: -350 lines, single source of truth for FFI declarations
- **Future addition cost**: 1 line in `BUILTINS` (vs. 5-7 lines in 3 places previously)
