# Ruyi — Agent Guidelines

## What This Is

Ruyi is a **compiled programming language** (JS-like syntax) targeting native machine code via LLVM. The compiler (`ruyic`) is written in Rust.

## Project Structure

```
ruyi/
├── crates/
│   ├── ruyic/           # Compiler crate (main binary + lib)
│   │   ├── src/
│   │   │   ├── main.rs      # CLI driver (clap)
│   │   │   ├── lib.rs       # Public API
│   │   │   ├── driver.rs    # Compilation pipeline orchestrator
│   │   │   ├── lexer/       # Tokenizer
│   │   │   ├── parser/      # AST parser
│   │   │   ├── macro_expand/ # Declarative macro system
│   │   │   ├── typechecker/ # Gradual type checker + inference
│   │   │   ├── codegen/     # LLVM IR generation (inkwell)
│   │   │   ├── gc/          # Garbage collector (generational)
│   │   │   ├── runtime/     # Runtime support
│   │   │   └── diagnostics/ # Error reporting
│   │   └── tests/       # Integration + unit tests per module
│   └── ruyi_runtime/    # Runtime library (GC, async, exceptions)
├── stdlib/              # Standard library (.ry source files)
├── examples/            # Example .ry programs
├── docs/
│   ├── spec.md          # Language specification (authoritative)
│   └── tutorial.md      # User tutorial
└── Cargo.toml           # Workspace root
```

## Compilation Pipeline

`driver.rs` orchestrates: **Source → Lexer → Parser → Macro Expansion → TypeChecker → CodeGen (LLVM) → Linker**

CLI flags: `-o <output>`, `--emit-llvm`, `--emit-ast`, `--emit-typed-ast`, `--check`, `-O0/-O1/-O2`, `--debug`

## Developer Commands

```bash
# Full workspace build (requires LLVM 14-18)
cargo build --release          # Binary at ./target/release/ruyic
cargo build -p ruyic           # Debug build of compiler only

# Check without linking (faster)
cargo check --workspace

# Runtime-only check (no LLVM needed)
cargo check -p ruyi_runtime --no-default-features

# Run tests
cargo test --workspace

# Run a single test file
cargo test -p ruyic --test typechecker

# Lint
cargo clippy --workspace

# Format
cargo fmt

# Compile a .ry file
ruyic examples/hello.ry -o hello && ./hello
ruyic examples/hello.ry --emit-llvm   # Output LLVM IR
ruyic examples/hello.ry --check       # Type-check only
```

## Setup Requirements

- **LLVM 14-18 is required** for the full build (inkwell binding). Without it, `cargo build` fails on `llvm-sys`.
  - macOS: `brew install llvm@17` then set `LLVM_SYS_170_PREFIX`
  - Runtime-only development: `--no-default-features` on `ruyi_runtime` skips inkwell
- Rust 2021 edition, workspace resolver = "2"

## Code Conventions

- **rustfmt**: 4-space tabs, max_width=100, Unix newlines
- **clippy**: warn-by-default enabled
- **Javadoc-style doc comments** on all public items (`/** ... */` with `@author`, `@date`)
- **Error types**: Use `thiserror` for derive, `anyhow` for application-level

## Testing

- Tests live alongside source: `crates/ruyic/tests/` (one file per module: `lexer.rs`, `parser.rs`, `typechecker.rs`, etc.)
- Runtime tests: `crates/ruyi_runtime/tests/`
- Integration test fixtures in `crates/ruyic/tests/integration/`
- Benchmarks: `crates/ruyic/benches/` (criterion)

## Language Quick Reference (.ry files)

- Keywords: `let`, `const`, `fn`, `class`, `trait`, `match`, `if`, `else`, `for`, `while`, `return`, `throw`, `try`, `catch`, `finally`, `async`, `await`, `import`, `export`, `macro`, `type`
- No `var`, no `undefined`, no `==`/`!=` (strict `===`/`!==` only), no `function` (use `fn`)
- Methods use `self` (not `this`)
- Nullable types explicit: `string?`, null assertion: `value!`
- Built-in types: `int` (i64), `float` (f64), `bool`, `string`, `null`, `void`, `dyn`, `never`, `bigint`
- Semicolons required (stricter ASI than JS)

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `inkwell` | LLVM bindings (llvm14-0 feature) |
| `clap` | CLI parsing (derive) |
| `thiserror` / `anyhow` | Error handling |
| `log` / `env_logger` | Logging |
| `criterion` | Benchmarking |

## Authoritative Sources

- Language spec: `docs/spec.md`
- Tutorial: `docs/tutorial.md`
- Compiler pipeline: `crates/ruyic/src/driver.rs`
- CLI entry: `crates/ruyic/src/main.rs`
