# Spec: LandingPadGenerator Shared Crate Refactor

## ADDED Requirements

### REQ-LPG-001: Workspace MUST include `ruyi_exception` shared crate

A new workspace member crate `crates/ruyi_exception` MUST be created to host the `LandingPadGenerator` and related exception-handling types. This crate MUST be added to the root `Cargo.toml` `[workspace.members]` list.

The crate MUST be a leaf-level shared library, depended on by both `ruyic` (compiler) and `ruyi_runtime` (runtime).

#### Scenario: ruyi_exception is a workspace member
- **WHEN** the root `Cargo.toml` is read
- **THEN** `crates/ruyi_exception` is listed under `[workspace.members]`

#### Scenario: cargo check builds ruyi_exception alongside other crates
- **WHEN** `cargo check --workspace` is executed
- **THEN** the build succeeds and `ruyi_exception` is compiled as part of the workspace

### REQ-LPG-002: ruyi_exception MUST expose LandingPadGenerator publicly

`LandingPadGenerator` (moved from `ruyi_runtime/src/exception/landing_pad.rs`) MUST be re-exported from `ruyi_exception::landing_pad` with the same public API. Both `ruyic` and `ruyi_runtime` MUST be able to construct instances and call methods.

The crate MUST depend on `inkwell` with the `llvm14-0` feature (gated behind a cfg so that no-LLVM builds of `ruyi_exception` still compile for the runtime-only build mode).

#### Scenario: ruyic imports LandingPadGenerator from ruyi_exception
- **WHEN** `crates/ruyic/src/codegen/stmt.rs` adds `use ruyi_exception::landing_pad::LandingPadGenerator;`
- **THEN** `cargo check -p ruyic` compiles successfully

#### Scenario: ruyi_runtime still imports LandingPadGenerator
- **WHEN** `crates/ruyi_runtime/src/exception/*` updates `use` statements to point to `ruyi_exception::landing_pad::LandingPadGenerator`
- **THEN** `cargo check -p ruyi_runtime` compiles successfully

#### Scenario: Runtime-only build (no LLVM) still compiles ruyi_runtime
- **WHEN** `cargo check -p ruyi_runtime --no-default-features` is executed
- **THEN** the build succeeds (the `inkwell`/`llvm14-0` feature is gated in `ruyi_exception` such that the runtime-only path doesn't require LLVM)

## MODIFIED Requirements

### REQ-LPG-003: LandingPadGenerator MUST be removed from ruyi_runtime::exception

The `LandingPadGenerator` struct and its `impl` block MUST be removed from `crates/ruyi_runtime/src/exception/landing_pad.rs` (or made a re-export from `ruyi_exception`). Any other type that `LandingPadGenerator` depended on (e.g. `TypeId` and exception interfaces) MUST also be migrated, OR `LandingPadGenerator` MUST be modified to use raw integer type IDs to break the dependency on runtime-specific types.

#### Scenario: ruyi_runtime::exception no longer contains LandingPadGenerator body
- **WHEN** the file `crates/ruyi_runtime/src/exception/landing_pad.rs` is inspected
- **THEN** it either (a) is removed entirely, or (b) contains only `pub use ruyi_exception::landing_pad::*` re-exports

#### Scenario: No public API surface regresses
- **WHEN** `cargo doc --workspace --no-deps` is executed
- **THEN** the documented public API of `ruyi_runtime::exception` matches the pre-change API for all existing public items except `LandingPadGenerator` (which now lives in `ruyi_exception`)

### REQ-LPG-004: workspace dependencies MUST be reorganized

The root `Cargo.toml` and individual crate `Cargo.toml` files MUST be updated to:
- Add `crates/ruyi_exception` to workspace members
- Add `ruyi_exception` as a workspace dependency
- Make `ruyic` and `ruyi_runtime` both depend on `ruyi_exception` (where applicable)
- Existing dependencies (`inkwell`, `llvm-sys`, etc.) MUST remain transitively reachable via `ruyi_exception`

#### Scenario: cargo build --workspace succeeds with no warnings
- **WHEN** `cargo build --workspace` is executed (with LLVM 14 prefix set)
- **THEN** the build succeeds with no `unused dependency` warnings

#### Scenario: cargo tree shows ruyi_exception as a transitive dep
- **WHEN** `cargo tree -p ruyic` is executed
- **THEN** `ruyi_exception` appears in the output (confirming the dependency graph)

### REQ-LPG-005: TRY_CATCH_AUDIT.md MUST be updated to reflect new architecture

After the refactor, the `TRY_CATCH_AUDIT.md` file's §5 summary table MUST be updated to reflect the new state:

| Question | Before | After |
|----------|--------|-------|
| Is LandingPadGenerator compatible with codegen? | NO | YES |
| Is LandingPadGenerator accessible from ruyic? | NO | YES (via ruyi_exception shared crate) |
| Does compile_try use invoke? | NO | YES |

#### Scenario: Updated audit document answers questions accurately
- **WHEN** the updated `TRY_CATCH_AUDIT.md` §5 table is read
- **THEN** all three answers in the table match the post-implementation reality
