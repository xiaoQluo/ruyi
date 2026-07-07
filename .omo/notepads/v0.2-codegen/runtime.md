# Runtime Functions (v0.2)

## What Was Added

- `crates/ruyi_runtime/src/builtins.rs` — 5 new C-ABI runtime functions:
  - `ruyi_string_concat(lhs, rhs) -> *mut i8`
  - `ruyi_array_alloc(capacity) -> *mut i8`
  - `ruyi_object_alloc(field_count) -> *mut i8`
  - `ruyi_bigint_from_str(s) -> *mut i8`
  - `ruyi_member_access(obj, offset) -> *mut i8`

- `crates/ruyi_runtime/tests/builtins.rs` — integration smoke tests for all 5 functions
- `crates/ruyi_runtime/src/lib.rs` — exports the new `builtins` module
- `crates/ruyi_runtime/tests/runtime.rs` — fixed missing `#[cfg(feature = "inkwell")]` on `test_ruyi_context_inkwell_types`

## Design Decisions

- **Allocator**: Used `std::alloc::alloc` / `dealloc` (system allocator) instead of `libc::malloc` because it is idiomatic Rust and consistent with the existing `alloc.rs` module. This satisfies the "simple malloc/free" requirement conceptually.
- **String layout**: Null-terminated C strings (simplest opaque representation).
- **Array layout**: `[len: i64][cap: i64][data: *mut i8 × cap]`
- **Object layout**: `[field_count: i64][fields: *mut i8 × field_count]`
- **BigInt layout**: Staged implementation — just a heap-allocated string copy. Real arbitrary-precision math deferred.
- **Member access**: Skips the leading `i64` header and indexes into the `*mut i8` slots that follow.

## Edge Cases Handled

- Negative capacity / field_count → clamped to 0 for allocation, but header stores the clamped value (non-negative).
- Null input pointers → return null or empty string as appropriate.
- Member access with null object or negative offset → returns null.

## Testing

- 10 unit tests in `builtins.rs`
- 5 integration smoke tests in `tests/builtins.rs`
- All pass with `cargo test -p ruyi_runtime --no-default-features`
- `tests/runtime.rs` inkwell test was missing its `cfg` gate; fixing it allows the whole no-default-features suite to pass.

## Open Questions / Deferred

- BigInt is not a real arbitrary-precision number yet.
- No destructor / free helper for these allocations (manual `dealloc` required).
- GC integration deferred per task instructions.
