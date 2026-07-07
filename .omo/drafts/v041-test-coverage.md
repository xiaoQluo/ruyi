# Draft: v0.4.1 Complete Test Coverage

## Requirements (confirmed)
- **Test depth**: Complete Coverage (Recommended) - All 7 gap areas + edge cases + cross-feature
- **Test format**: Both (Recommended) - Integration .ry test files AND Rust runtime unit tests
- **目录约束** (NEW): .ry 文件放 `test/`, 编译输出放 `test/target/`

## Gap Analysis
### Integration .ry tests needed:
1. for-in loop tests
2. for-of loop tests
3. Optional chaining (?.) tests
4. Computed member (obj[expr]) tests
5. Template literal tests
6. impl Trait for built-in tests
7. Enhanced match tests (edge cases)

### Runtime unit tests needed:
8. Thread-local GC heap tests
9. Async GC roots tests (GenerationalCollector)
10. Exception landing pad integration tests

### Edge cases needed:
11. Nested patterns (match + loops, match + exceptions)
12. Cross-feature combinations
13. Error path tests (invalid inputs)

## Technical Decisions
- Test format: Follow existing patterns (tests/integration/cases/ + .expected files)
- LLVM status: No LLVM 14 in environment → use --emit-llvm for codegen validation, --check for type checking
- Runtime tests: cargo test -p ruyi_runtime --no-default-features

## Scope Boundaries
- INCLUDE: All 7 gap areas, edge cases, cross-feature tests
- EXCLUDE: Standard library tests (v0.5+), already-tested features, LLVM execution tests

## Test Infrastructure
- Integration: tests/integration/runner.rs (discover .ry → compile → run → compare .expected)
- Runtime: crates/ruyi_runtime/tests/*.rs (standard Rust #[test])
- Codegen: tests/codegen.rs (compile only, compare --emit-llvm output)