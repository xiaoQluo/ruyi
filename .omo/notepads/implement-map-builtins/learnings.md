# Learnings: Implement map builtins

## Patterns observed
- C-ABI runtime functions use `#[no_mangle] pub extern "C" fn` with raw pointers.
- All heap allocations go through `ruyi_gc_alloc(size: i64) -> *mut u8`.
- Array layout: `[len: i64][cap: i64][data: *mut i8 * cap]`.
- Object layout: `[field_count: i64][fields: *mut i8 * field_count]`.
- BuiltinRegistry in codegen maps `__builtin_*` names to runtime C symbols.
- Javadoc-style doc comments are required on all public items.

## Successful approaches
- Implementing a custom hash table with explicit memory layout (header + buckets + chained entries) instead of Rust's HashMap to ensure C-ABI compatibility.
- Using FNV-1a hash for string keys is simple and effective for this use case.
- Auto-resizing when load factor exceeds 0.75 keeps performance reasonable.
- Reusing existing `ruyi_array_alloc`/`push`/`set`/`get` for keys/values/entries avoids duplicating array logic.

## Map layout chosen
- Header (24 bytes): `[entry_count: i64][bucket_count: i64][buckets_ptr: *mut *mut u8]`
- Entry (32 bytes): `[hash: u64][key: *mut i8][value: *mut i8][next: *mut u8]`
