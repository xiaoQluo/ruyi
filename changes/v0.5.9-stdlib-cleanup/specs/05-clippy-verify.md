# Spec 05: clippy-verify — Zero-new-clippy snapshot diff verification

## Overview

`v0.5.8-stdlib-cleanup` baseline has 52 errors / 32 warnings from `cargo clippy --workspace` (mostly in `ruyi_runtime::gc`, pre-existing since v0.5.5). This spec **forbids v0.5.9 from adding any new lints** but does not require resolving the pre-existing ones (NG-1 in `proposal.md`).

Acceptance is a structural diff between the v0.5.8 baseline clippy output and the v0.5.9 clippy output. Empty diff = R4 passes.

## Requirements

### REQ-1: Snapshot baseline
**SHALL** on the `v0.5.8` tag, capture:
```bash
git checkout v0.5.8
cargo clean -p ruyi_runtime
cargo clippy --workspace 2>&1 | sort > /tmp/v058_clippy_baseline.txt
```

**Note**: `cargo clean -p ruyi_runtime` ensures the runtime crate is freshly built so the clippy output is reproducible.

### REQ-2: Snapshot current
**SHALL** on `dev/v0.5.9-stdlib-cleanup` (after T1-T4 commits), capture:
```bash
git checkout dev/v0.5.9-stdlib-cleanup
cargo clean -p ruyi_runtime
cargo clippy --workspace 2>&1 | sort > /tmp/v059_clippy.txt
```

### REQ-3: Diff verification
**SHALL** run:
```bash
diff /tmp/v058_clippy_baseline.txt /tmp/v059_clippy.txt > /tmp/clippy_diff.txt
```

**Acceptance**: `/tmp/clippy_diff.txt` is empty (or contains only known false positives from reformatting — e.g., line number shifts in unmodified files).

### REQ-4: Fix any new lints
**SHALL** for any new lint in the diff:
- If the lint is in a file modified by T1-T4: fix the source
- If the lint is in a file NOT modified by T1-T4: investigate (this would be a flaky clippy behavior, unlikely)
- If the lint is `#[allow(...)]`-eligible and the fix is non-trivial: add `#[allow(...)]` with a 1-line justification comment

**Iterate**: re-run REQ-1 through REQ-3 until diff is empty.

## Scenarios

### SCEN-1: Empty diff
**WHEN** v0.5.9 introduces no new clippy lints vs v0.5.8 baseline
**THEN** `diff /tmp/v058_clippy_baseline.txt /tmp/v059_clippy.txt` produces no output

**Acceptance**: empty diff

### SCEN-2: One new lint found
**WHEN** T2 (R5 codegen table refactor) introduces a single `dead_code` warning in `codegen/builtins_table.rs::sig_to_fn_type`
**THEN** add `#[allow(dead_code)]` with justification, re-run, diff is empty

**Acceptance**: diff empty after fix

### SCEN-3: Lint count matches exactly
**WHEN** the lints are otherwise identical (no new, no removed)
**THEN** both files have exactly 84 lines (52 errors + 32 warnings) and they sort to identical content

**Acceptance**: `diff` exit 0; `wc -l` matches

## Out of Scope

- Fixing pre-existing 52 errors / 32 warnings (NG-1 in `proposal.md`)
- Adding new clippy lints to the codebase (R4 forbids new, not their existence)
- Adding `#![deny(warnings)]` or similar enforcement (separate quality change)
- Setting up automated clippy CI (separate infra change)

## Risks

| ID | Risk | Mitigation |
|----|------|------------|
| R4-1 | A `cargo clean -p ruyi_runtime` does not actually rebuild (e.g., cargo cache is wrong) → clippy output is non-reproducible | Run `cargo clean --workspace` before snapshot if suspicious |
| R4-2 | Flaky lints (e.g., line-number-dependent warnings) cause spurious diffs | Sort both files identically; manual review of any diff lines |
| R4-3 | R5's table refactor introduces a lint that's hard to fix in the table structure (e.g., a `dead_code` on a static slice) | Use `#[allow(...)]` with justification; document in commit message |
| R4-4 | A pre-existing lint happens to fire on a different line number in v0.5.9 (e.g., due to formatting changes) | Normalize by sorting; if a line number is the only diff, investigate |

## Verification Command (one-liner)

After T1-T4 commits on `dev/v0.5.9-stdlib-cleanup`:

```bash
git checkout v0.5.8 && cargo clean -p ruyi_runtime && cargo clippy --workspace 2>&1 | sort > /tmp/v058.txt && \
git checkout dev/v0.5.9-stdlib-cleanup && cargo clean -p ruyi_runtime && cargo clippy --workspace 2>&1 | sort > /tmp/v059.txt && \
diff /tmp/v058.txt /tmp/v059.txt && echo "R4 PASS: zero new clippy lints"
```

If the diff is non-empty, the output is the list of new lints to fix.
