## v0.5.1 Bugfix Session - Learnings and Issues

### Successful Fixes
1. **Typechecker: Function parameter scope** - Parameters must be declared BEFORE `infer_return_type` is called, otherwise return expressions can't resolve parameter names
2. **Typechecker: Two-pass function collection** - First pass collects all function declarations (signatures), enabling forward references. Second pass does full type inference
3. **Codegen: Arrow function support** - Added `Expr::ArrowFunction` handling with unique name generation (`__arrow_N`)
4. **Parser: If-expression semicolons** - Made semicolons optional in if-expression blocks (when followed by `}`)
5. **Codegen: If-expression support** - Added `compile_if_expr` with phi nodes for merge values
6. **Codegen: Trait method fallback** - Method calls now try `ClassName_methodName` first, then fall back to `methodName_*_for_ClassName` pattern for trait impls

### Root Causes of Remaining Failures
1. **Stdlib not auto-included** - Examples rely on `toString`, `Error`, `Array` methods from stdlib, but stdlib is not automatically imported
2. **Generic method calls** - `Option<U>.new(...)` fails because parser treats `<U>` as comparison, not type args
3. **Pattern matching type errors** - Array pattern binding doesn't properly extract element types
4. **try_catch SIGILL** - Runtime exception handling support incomplete (landing pads, personality functions)
5. **Power operator** - Unimplemented binary operator
6. **Generator syntax** - `fn*` not implemented
7. **--test flag** - Not implemented in CLI

### Key Insights
- The compiler has a clean separation between parser, typechecker, and codegen
- Typechecker uses a scoped environment with push/pop for block scoping
- Codegen uses inkwell for LLVM IR generation
- Trait impl methods are mangled as `methodName_traitName_for_typeName`
- The stdlib is resolved on-demand via imports, not auto-included

### Verification Results
- 9 examples PASS (hello, fibonacci, float_math, compare_test, ternary, array, async, generics, generics_simple)
- 16 examples EXP_FAIL (stdlib dependencies, unimplemented features)
- 0 examples FAIL (unexpected)
