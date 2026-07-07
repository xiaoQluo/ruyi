
## F4 Re-run Findings (2026-05-04)

### Key Patterns Observed
- `crates/ruyi_runtime/src/regex.rs` correctly uses `Lazy<Mutex<HashMap<i64, Regex>>>` for GC-compatible handle management.
- `crates/ruyic/src/codegen/builtins.rs` still uses the old per-function `declare_*` pattern; no `BuiltinRegistry` infrastructure exists anywhere in the compiler.
- The lexer unconditionally maps `"match"` → `Token::Match` (scanner.rs:445), making it impossible to use as a property/method name.

### Blockers / Gotchas
- T15 fix claim is false: `ruyi_process_exec` / `ruyi_process_exec_with` exist in runtime (`process.rs`) but have ZERO compiler-side declarations. Any .ry code calling `__process_exec` will fail at codegen/link.
- T26 fix claim is false: `chain`, `enumerate`, `zip`, `sum`, `product` are completely absent from `stdlib/collections.ry` and all stdlib files.
- T32 verification claim is false: `core.ry` (module String) and `string.ry` (free functions) share 10+ duplicate method names (split, startsWith, endsWith, contains, replace, toUpperCase, toLowerCase, trim, length, slice).
- T29 fix introduces a new syntax error: `fn match(self, text: string)` uses reserved keyword `match` which parser's `parse_property_name` does not accept.

### Unresolved Technical Debt
- No BuiltinRegistry in compiler = all new stdlib builtins (math, time, json, random, fmt, regex, buffer, net, process) are unwired.
- `Array<T>` still has recursive field `let mut elements: Array<T>` (Task 3 from plan, never fixed).
