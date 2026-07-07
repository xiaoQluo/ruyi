# Decisions: Implement map builtins

## Architecture choices
1. **Chaining over open-addressing**: Chosen because deletion is simpler and more robust with chaining. Open-addressing deletion requires tombstones or rehashing.
2. **Store key pointers directly instead of copying**: Keys are assumed to be immutable GC-managed strings. Copying would add unnecessary allocations.
3. **Auto-resize at load factor 0.75**: Standard threshold that balances memory and performance.
4. **Set returns void**: The stdlib (`collections.ry`) uses `__builtin_map_set(...)` as a statement and ignores the return value.
5. **Delete returns i64 (1/0) instead of bool**: Matches existing builtin conventions (e.g., `ruyi_map_has` returns `i64`).
6. **Entries return arrays of [key, value] pairs**: Each pair is a standard Ruyi array of length 2, consistent with the existing array runtime.
