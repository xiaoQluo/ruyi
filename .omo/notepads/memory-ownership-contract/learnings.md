## 2026-05-04: Memory Ownership Contract for builtins

### Changes Made
- Replaced all `std::alloc::alloc(layout)` calls → `ruyi_gc_alloc(layout.size() as i64)` in 6 functions
- Replaced `std::alloc::realloc` in `ruyi_array_push` → new GC alloc + `copy_nonoverlapping` pattern
- Added module-level memory ownership contract documentation
- Added `## Ownership:` doc line to each C export function
- Removed all `dealloc` calls from tests (GC manages memory now)
- Both `tests/builtins.rs` (integration) and `src/builtins.rs` (unit) test files cleaned up

### Key Pattern
Replace: `alloc(layout) as *mut i8` → `ruyi_gc_alloc(layout.size() as i64) as *mut i8`
Replace realloc: `std::alloc::realloc(arr, old_layout, new_size)` → `ruyi_gc_alloc(new_size)` + `copy_nonoverlapping` old data

### Verification
- `cargo test -p ruyi_runtime --no-default-features` passes all builtins tests (13 unit + 5 integration)
- GC async root failures are pre-existing and unrelated
