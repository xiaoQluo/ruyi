# F3 QA Issues

## Issue 1: Member Access Not Supported
**Severity**: High
**Description**: Codegen fails when accessing object members like `o.x`
**Error**: `codegen error: Unsupported expression: Member { object: Identifier("o"), property: Ident("x"), optional: false }`
**Impact**: Cannot access fields on objects returned from functions

## Issue 2: Multiple Async Functions Output Issue
**Severity**: Low
**Description**: Multiple async functions compile but running produces no output
**Impact**: Async machinery works but return values aren't properly handled when assigned to variables

## Issue 3: Nested Try/Catch/Finally Linking
**Severity**: Medium
**Description**: Nested try/catch/finally compiles but linking fails with missing temp object file
**Error**: `clang: error: no such file or directory: '/var/folders/.../ruyi_temp.o'`
**Impact**: Complex exception patterns with finally blocks cannot link

## Issue 4: LLVM Not Available for Tests
**Severity**: High (Environment)
**Description**: LLVM 14 not configured, cargo tests fail to compile `llvm-sys`
**Impact**: Cannot run unit/integration tests locally
**Note**: The pre-built binary works fine - this is a test environment issue only