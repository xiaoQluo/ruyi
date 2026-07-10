# Spec: Compiler Static Linking of `ruyi_runtime`

## MODIFIED Requirements

### REQ-LINK-001: Driver MUST link `libruyi_runtime.a` when `--gc=real`

The `driver.rs` MUST statically link `libruyi_runtime.a` into the output binary when `--gc=real` is specified. With `--gc=stub` (default), the driver MUST continue using the bare `cc` flow (no `ruyi_runtime` linking) to preserve v0.5.5 behavior.

#### Scenario: --gc=stub preserves v0.5.5 linking behavior
- **WHEN** user runs `ruyic examples/hello.ry -o hello` without `--gc`
- **THEN** `ldd ./hello | grep ruyi_runtime` returns nothing AND binary behavior matches v0.5.5 baseline

#### Scenario: --gc=real links ruyi_runtime statically
- **WHEN** user runs `ruyic --gc=real examples/fib.ry -o fib`
- **THEN** `ldd ./fib` shows no ruyi_runtime dynamic reference; binary size increased by ruyi_runtime static library footprint (200KB-1MB)

### REQ-LINK-002: `ruyi_runtime` MUST be pre-built as static library

`cargo build -p ruyi_runtime --release` MUST produce `target/release/libruyi_runtime.a` (Linux) or platform-equivalent static archive.

#### Scenario: cargo build produces static archive
- **WHEN** `cargo build -p ruyi_runtime --release` is executed
- **THEN** `target/release/libruyi_runtime.a` (or platform equivalent) exists and is non-empty

#### Scenario: ruyi_runtime builds without LLVM dependency for stub features
- **WHEN** `cargo build -p ruyi_runtime --release --no-default-features` is executed
- **THEN** the build succeeds without requiring LLVM 14 (only GC/exception/async modules, no codegen-landing-pad generation)

### REQ-LINK-003: Static linking MUST NOT regress existing examples

The 33 existing examples MUST continue to pass when compiled with `--gc=real`.

#### Scenario: All examples pass in real mode
- **WHEN** `examples/run_examples.sh` is extended to also test `--gc=real` mode
- **THEN** all 33 examples still produce the expected output