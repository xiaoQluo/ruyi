
## 2026-05-04: Fixed 4 F4 Scope Fidelity Issues

### T15: Added __process_exec / __process_exec_with to builtins.rs
- Added `declare_ruyi_process_exec` and `declare_ruyi_process_exec_with` functions
- Added calls in `declare_builtins()`
- Followed exact pattern of existing declarations (i8_ptr return, matching runtime signatures)
- `ruyi_process_exec` takes single i8_ptr arg, returns i8_ptr
- `ruyi_process_exec_with` takes 4 args (i8_ptr, i8_ptr, i8_ptr, i64), returns i8_ptr

### T26: Added Iterator methods to collections.ry
- Added `chain`, `enumerate`, `zip`, `sum`, `product` methods to `trait Iterator<T>`
- Added supporting classes: `ChainIterator<T>`, `EnumerateIterator<T>`, `ZipIterator<T, U>`
- Each implements `Iterator` trait with proper `next()` method
- Used Ruyi syntax conventions: `while (item !== null)` loops, null assertion `item!`

### T29: Renamed match() to findMatch() in stdlib/regex.ry
- Renamed `fn match(self, text: string)` to `fn findMatch(self, text: string)`
- `match` is a reserved keyword in Ruyi, so this was required
- Internal `__builtin_regex_find` call unchanged
- No callers needed updating (test file uses builtins directly)

### T32: Removed duplicate String methods from core.ry
- Removed entire `module String` block from stdlib/core.ry
- Removed 10 duplicate methods: length, slice, replace, toUpperCase, toLowerCase, trim, contains, startsWith, endsWith, split
- Canonical implementations remain in stdlib/string.ry
- core.ry now only contains Int, Float, Bool modules

### Verification
- ruyi_runtime checked with `cargo check -p ruyi_runtime --no-default-features` (passed)
- Full workspace check blocked by missing LLVM environment (expected)
- All .ry file changes are syntactically consistent with existing code patterns
