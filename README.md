# Ruyi（如意）

> A compiled, general-purpose programming language built on the syntactic foundation of JavaScript strict mode — targeting native machine code via LLVM.

[中文文档](README-zh.md)

## Overview

Ruyi removes problematic JavaScript features while retaining familiar syntax. It compiles to native machine code via LLVM, providing high performance across platforms.

## Key Features

- **Familiar syntax** — If you know JavaScript, you already know most of Ruyi's syntax.
- **Compiled to native code** — Uses LLVM to produce fast, standalone binaries.
- **Gradual typing** — Choose between static type annotations and dynamic typing (`dyn`).
- **Null safety** — No `undefined`. Nullable types are explicit (`string?`).
- **Pattern matching** — First-class `match` expressions with destructuring and guards.
- **Traits** — Interface-like contracts with static and dynamic dispatch (`dyn Trait`).
- **Generics** — Parametric polymorphism with monomorphization for zero runtime overhead.
- **Async/await** — Green-thread-based concurrency with a work-stealing scheduler.
- **Exception handling** — Zero-cost try/catch/finally via LLVM landing pads.
- **Macros** — Declarative, hygienic compile-time code generation.

## Quick Start

### Prerequisites

- **Rust** (2021 edition)
- **LLVM 14** (required for full build)
  - macOS: `brew install llvm@14` then set `LLVM_SYS_140_PREFIX`
  - Linux: Install via your package manager (e.g. `apt install llvm-14-dev`)

### Build from Source

```bash
git clone https://github.com/xiaoQluo/ruyi.git
cd ruyi
cargo build --release
```

The compiler binary will be at `./target/release/ruyic`.

### Hello, World!

Create `hello.ry`:

```ruyi
print("Hello, Ruyi!");
```

Compile and run:

```bash
./target/release/ruyic hello.ry -o hello
./hello
```

## Compiler Usage

```bash
ruyic <input> [options]
```

| Flag | Description |
|------|-------------|
| `-o <output>` | Specify output binary name |
| `--emit-llvm` | Output LLVM IR instead of a binary |
| `--emit-ast` | Output AST (for debugging) |
| `--emit-typed-ast` | Output typed AST (for debugging) |
| `--check` | Parse and type-check only (no codegen) |
| `-O0`, `-O1`, `-O2` | Optimization level (default: `-O0`) |
| `--debug` | Include debug symbols |
| `--version` | Print compiler version |

## Project Structure

```
ruyi/
├── crates/
│   ├── ruyic/              # Compiler (binary + lib)
│   │   ├── src/
│   │   │   ├── main.rs     # CLI driver (clap)
│   │   │   ├── driver.rs   # Compilation pipeline orchestrator
│   │   │   ├── lexer/      # Tokenizer
│   │   │   ├── parser/     # AST parser
│   │   │   ├── macro_expand/  # Declarative macro system
│   │   │   ├── typechecker/   # Gradual type checker + inference
│   │   │   ├── codegen/    # LLVM IR generation (inkwell)
│   │   │   ├── gc/         # Garbage collector
│   │   │   ├── runtime/    # Runtime support
│   │   │   └── diagnostics/# Error reporting
│   │   └── tests/          # Per-module tests + integration cases
│   └── ruyi_runtime/       # Runtime library (GC, async, exceptions)
├── stdlib/                  # Standard library (.ry source files)
├── examples/                # Example .ry programs
└── docs/
    ├── spec.md              # Language specification
    └── tutorial.md          # User tutorial
```

## Developer Commands

```bash
# Build
cargo build --release              # Release binary
cargo build -p ruyic               # Debug build

# Check (faster, no linking)
cargo check --workspace
cargo check -p ruyi_runtime --no-default-features  # Runtime-only (no LLVM)

# Test
cargo test --workspace
cargo test -p ruyic --test typechecker   # Single test file

# Lint & Format
cargo clippy --workspace
cargo fmt
```

## Language Highlights

### What's Different from JavaScript

| JavaScript | Ruyi | Reason |
|------------|------|--------|
| `var x` | `let x` | Block-scoped, not function-scoped |
| `undefined` | `null` | Single null-like value |
| `==` / `!=` | `===` / `!==` | Strict equality only, no coercion |
| `function() {}` | `fn() {}` | Shorter keyword |
| `this` in methods | `self` | Explicit, no binding confusion |
| `arguments` | `...args` | Rest parameters |
| `prototype` | `class` / `trait` | Class-based inheritance |
| `with` | _(removed)_ | No equivalent |
| `eval()` | _(removed)_ | No equivalent |
| `delete obj.prop` | `obj.prop = null` | No property deletion |

### Built-in Types

| Type | Description |
|------|-------------|
| `int` | 64-bit signed integer |
| `float` | 64-bit floating point |
| `bool` | Boolean (`true` / `false`) |
| `string` | UTF-8 string |
| `null` | Null type (only value: `null`) |
| `void` | No return value |
| `dyn` | Dynamic type (runtime checked) |
| `never` | Bottom type (unreachable) |
| `bigint` | Arbitrary precision integer |

### Example: Fibonacci

```ruyi
fn fib(n: int): int {
  if (n <= 1) {
    return n;
  }
  return fib(n - 1) + fib(n - 2);
}

fn main() {
  for (let i = 0; i < 10; i = i + 1) {
    print("fib(" + i + ") = " + fib(i));
  }
}
```

### Example: Classes and Traits

```ruyi
trait Printable {
  fn format(self): string;
}

class Point {
  x: float;
  y: float;

  fn new(x: float, y: float) {
    self.x = x;
    self.y = y;
  }
}

impl Printable for Point {
  fn format(self): string {
    return "(" + self.x + ", " + self.y + ")";
  }
}
```

## Documentation

- [Language Specification](docs/spec.md) — Authoritative reference
- [Tutorial](docs/tutorial.md) — Step-by-step guide
- [Roadmap](docs/roadmap.md) — Development roadmap and future plans
- [中文规范](docs/spec-zh.md) — 语言规范（中文版）
- [中文教程](docs/tutorial-zh.md) — 教程（中文版）
- [中文路线图](docs/roadmap-zh.md) — 开发路线图（中文版）

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
