# F3 QA Problems (Unresolved)

## Problem 1: Member Access Codegen
**Status**: Open
**Description**: Object member access (`o.x`) is not implemented in codegen. This blocks basic object-oriented patterns.
**Required Fix**: Implement Member expression codegen in `crates/ruyic/src/codegen/`

## Problem 2: Async Return Value Handling
**Status**: Open
**Description**: When async functions return values assigned to variables, the values don't propagate correctly at runtime.
**Required Fix**: Investigate how FutureBox and return values are handled in codegen

## Problem 3: Temp File Cleanup Race
**Status**: Open
**Description**: Linker fails with missing temp object file - possible race condition in temp file management
**Required Fix**: Investigate temp file lifecycle in driver.rs or linker invocation