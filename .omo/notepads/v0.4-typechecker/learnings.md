# v0.4 Notepad

## [2026-05-03] Wave 1 Complete
- T1-T7 all implemented, 163/168 typechecker tests pass (5 pre-existing failures)
- LLVM_SYS_140_PREFIX=/usr/local/opt/llvm@14 required for cargo commands
- `Declaration::Trait` now has `supertraits: Vec<String>` field
- `TypeInference` now has `trait_registry: TraitRegistry` field
- `MonomorphizationTracker` now has `trait_registry: Option<TraitRegistry>` field
- `check_bounds()` wired to TraitRegistry::check_bound()
- `narrow_for_condition()` handles instanceof and typeof
- `find_missing_cases()` improved for named types and arrays
- TraitRegistry gained `resolve_impl_method()`, `has_method()`, `validate_supertraits()`
- Commit: dd4808d

## Differences from main (merge conflict resolution area)
- codegen files show uncommitted changes from main merge conflict resolution
- Runtime Cargo.toml may have duplicate [lib] key from merge
- These are NOT v0.4 changes - they are artifacts from the main merge
