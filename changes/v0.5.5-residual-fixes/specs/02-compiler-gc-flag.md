# Spec: Compiler `--gc` Flag for Stub/Real GC Modes

## ADDED Requirements

### REQ-GC-001: Compiler MUST accept `--gc` flag

The compiler MUST accept `--gc=stub` or `--gc=real` flag to control GC mode:

- `--gc=stub` (default when flag absent): use placeholder allocator via raw `malloc`, fast compile, no GC
- `--gc=real`: link with `ruyi_runtime`'s generational GC, slower compile, real memory management

#### Scenario: Default mode is stub
- **WHEN** user runs `ruyic examples/hello.ry -o hello` without `--gc` flag
- **THEN** compiled binary uses stub allocator; output identical to v0.5.5 baseline behavior

#### Scenario: real mode triggers real GC linkage
- **WHEN** user runs `ruyic --gc=real examples/fib.ry -o fib`
- **THEN** compiled binary calls `ruyi_gc_alloc` / `ruyi_gc_collect` from `ruyi_runtime`; binary size +200KB-1MB

#### Scenario: Invalid GC mode rejected
- **WHEN** user runs `ruyic --gc=invalid examples/hello.ry`
- **THEN** compiler prints error to stderr and exits non-zero

### REQ-GC-002: `--gc=real` MUST trigger static linking of `ruyi_runtime`

The `--gc=real` flag MUST cause the driver to link `libruyi_runtime.a` into the output binary (instead of using bare `cc`).

#### Scenario: --gc=real produces single-file binary
- **WHEN** user runs `ruyic --gc=real examples/hello.ry -o hello` followed by `ldd target/hello | grep ruyi_runtime`
- **THEN** no dynamic library references to ruyi_runtime are found (binary is self-contained)

### REQ-GC-003: Stub allocator MUST be preserved as fallback

The current placeholder allocator path MUST remain functional as the `--gc=stub` mode, ensuring v0.5.5 baseline behavior is unchanged when `--gc` is absent.

#### Scenario: All 33 examples pass in stub mode
- **WHEN** `bash examples/run_examples.sh` is executed without `--gc=real`
- **THEN** `Total: 33 | Passed: 33 | Failed: 0`