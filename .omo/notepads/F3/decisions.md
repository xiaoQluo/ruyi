# F3 QA Decisions

## Decision 1: Test Binary vs Source Compilation
**Rationale**: The pre-built `target/debug/ruyic` binary was used for all tests rather than rebuilding. This saved time and avoided the LLVM 14 setup issue for testing itself.

## Decision 2: Edge Case Scope
**Rationale**: Edge cases focused on language constructs rather than performance/stress testing. The goal was functional verification, not load testing.

## Decision 3: LLVM IR Verification
**Rationale**: For async testing, examined `--emit-llvm` output to verify code generation rather than requiring full compilation. This confirms the state machine generation is working.