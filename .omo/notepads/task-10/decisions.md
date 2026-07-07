# Task 10 Decisions

## Architecture
- Separated type system into 6 modules: types, environment, diagnostics, constraints, inference, checker
- TypeInference walks the AST and produces a typed environment + diagnostics
- TypeChecker is the public API that orchestrates inference
- ConstraintSolver is available for future generic type inference but not yet wired into the main checker

## Type Representation
- Used `Box<Type>` for recursive types (Nullable, Array, Function return type)
- `TypeVar` with id+name for generic inference
- `ObjectField` with optional flag for structural subtyping
- `Type::Error` for error recovery to prevent cascading errors
- `Type::Dynamic` for gradual typing escape hatch

## Subtyping Rules (per spec Section 8.3)
- Reflexive: T <: T
- int <: float (widening)
- T <: T? (nullable supertype)
- Never <: T (bottom type)
- dyn ~ T (consistency, not strict subtyping)
- Object subtyping is structural: more fields <: fewer fields
- Function subtyping: contravariant params, covariant return
- Array<T> <: Array<U> if T <: U (covariant)

## Inference Strategy
- Unannotated variables default to `dyn`
- Annotated variables use the annotation as the type
- Literal inference: int literal → Int, float literal → Float, etc.
- Function return type inference from return statements
- Arrow function parameters are bound in a new scope
