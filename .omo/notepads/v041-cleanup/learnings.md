

---

# Task 11: Match Statement Code Generation

## Changes Made

### crates/ruyic/src/codegen/mod.rs
- Added `pub mod patterns;` to include the new patterns module

### crates/ruyic/src/codegen/patterns.rs (rewritten)
- Replaced stub `PatternCompiler` and `BlockBuilder` trait with direct `CodegenContext` integration
- Added `compile_match_stmt` as the main entry point for match statement codegen
- Implemented `compile_int_match`: uses LLVM `switch` instruction for int literal patterns
- Implemented `compile_bool_match`: uses LLVM `br` (conditional branch) for true/false patterns
- Implemented `compile_string_match`: uses `strcmp` runtime call + icmp chain for string literals
- Implemented `compile_nullable_match`: uses `ptrtoint` + `icmp eq` null check + conditional branch
- Implemented `compile_generic_match`: fallback that jumps to first wildcard/identifier arm
- Added `bind_pattern`: binds `Pattern::Identifier` and `Pattern::As` to stack slots via `alloca` + `store`
- Added `compile_arm_bodies`: compiles each arm's statements and adds branch to merge block

### crates/ruyic/src/codegen/stmt.rs
- Added `Statement::Match { value, arms }` arm in `compile_stmt` routing to `patterns::compile_match_stmt`
- Fixed pre-existing compilation error: added `use inkwell::values::BasicValue` for `as_basic_value_enum` method

### crates/ruyic/src/typechecker/patterns.rs (bug fixes)
- Fixed `pattern_covered_cases` for `Pattern::Literal` to use canonical string keys (`"true"`, `"false"`, `"null"`, etc.) instead of `format!("{:?}", expr)` which produced keys like `"BooleanLiteral(true)"` that `find_missing_cases` couldn't match
- Fixed `pattern_covered_cases` for `Pattern::Identifier` to also insert `"_"` so identifier patterns are recognized as exhaustive
- Fixed `find_missing_cases` to use separate messages for int, float, and string types

## Key Implementation Details

### Int Match (Switch)
```llvm
switch i64 %x1, label %int_arm_2 [
  i64 1, label %int_arm_0
  i64 2, label %int_arm_1
]
```
- Literal int patterns become switch cases
- First wildcard/identifier arm serves as default
- If no default, emits unreachable trap block

### Bool Match (Br)
```llvm
br i1 %flag1, label %bool_arm_0, label %bool_arm_1
```
- Direct conditional branch on boolean value
- Missing literal arm falls through to wildcard/identifier arm or merge

### Nullable Match (Null Check + Branch)
```llvm
%ptr_int = ptrtoint i8* %opt1 to i64
%is_null = icmp eq i64 %ptr_int, 0
br i1 %is_null, label %null_arm_0, label %null_arm_1
```
- Pointer types: ptrtoint + icmp eq 0
- Integer types: icmp eq 0 (sentinel, since nullable primitives are erased to inner type at LLVM level)
- null literal arm matches null; identifier arm binds non-null value

## QA Evidence
- `.sisyphus/evidence/task-11-match-int.ll.txt`: LLVM IR shows `switch` instruction with int cases
- `.sisyphus/evidence/task-11-match-bool-check.txt`: Compilation passes, LLVM IR shows `br i1`
- `.sisyphus/evidence/task-11-match-destructure-check.txt`: Compilation passes, LLVM IR shows null check + variable binding

## Pitfalls Encountered
- `patterns.rs` was not included in `codegen/mod.rs` (module not compiled)
- `BlockBuilder` trait in original stubs was incompatible with actual `CodegenContext` + inkwell `Builder`
- Pre-existing `stmt.rs` compilation error: `BasicValue` trait not imported for `as_basic_value_enum`
- Typechecker `pattern_covered_cases` had bugs where literal patterns used `{:?}` debug format keys that didn't match what `find_missing_cases` searched for
- `test_bool_patterns_with_wildcard` was already failing before our changes (pre-existing typechecker redundancy bug)
- Nullable primitives (e.g., `int?`) map to inner LLVM type (i64) with no null representation - codegen uses 0 sentinel
