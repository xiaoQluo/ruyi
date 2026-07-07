## 2026-05-04: Task 1 - Version string fix

- Found 3 occurrences of the old version string:
  1. `Cargo.toml:9` — workspace version `"0.4.1"`
  2. `crates/ruyic/src/main.rs:18` — clap command version `"0.4.1"`
  3. `crates/ruyic/src/main.rs:50` — fallback print string `"v0.1.0"` (was a bug, previous version)
- All three changed to `"0.5.0"` (with `v` prefix preserved in the print string: `"v0.5.0"`)
- LLVM 14 is installed at `/usr/local/opt/llvm@14`, need `LLVM_SYS_140_PREFIX` env var to build
- `cargo build -p ruyic` works with the prefix
- `ruyic --version` outputs `ruyic 0.5.0` ✅
