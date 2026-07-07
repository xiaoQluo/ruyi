
SCOPE FIDELITY CHECK
====================
Date: 2026-05-04
Branch: dev/v0.4.1
Base: dev/v0.4

TASKS COMPLIANT: 12/13

  T0  Version bump to 0.4.1                          DONE (committed)
  T1  for/for-in/for-of loop codegen                 DONE (stmt.rs: compile_for, compile_for_in, compile_for_of)
  T2  break/continue codegen                         DONE (stmt.rs: Statement::Break/Continue with loop_stack)
  T3  Optional chaining ?. + computed member         DONE (expr.rs: compile_optional_member_access, MemberProperty::Expr)
  T4  Template literal codegen                       MISSING — no Expr::TemplateLiteral handler in codegen/expr.rs
  T5  BigInt literal codegen                         DONE (expr.rs: compile_bigint_literal)
  T6  Audit try/catch codegen                        DONE (TRY_CATCH_AUDIT.md created)
  T7  Exception landing pad wiring                   DONE (runtime.rs: _Unwind_RaiseException, __cxa_begin/end_catch; stmt.rs: landingpad + build_invoke)
  T8  async/await true async                         DONE (async_runtime.rs: ruyi_await uses scheduler suspend)
  T9  async GC roots for GenerationalCollector        DONE (generational.rs: register_async_roots)
  T10 impl Trait for built-in types                  DONE (traits.rs, parser.rs, types.rs: TypeAnnotation::Builtin)
  T11 match statement codegen                        DONE (patterns.rs: compile_match_stmt with int/bool/string/nullable/generic dispatch)
  T12 Thread-local GC heaps                          DONE (gc_exports.rs: thread_local CURRENT_COLLECTOR)

Must NOT Have Violations:
- [x] Promise.all / Promise.race / Promise.any / Promise.allSettled: Not found in new code
- [x] Labeled break / labeled continue: Not found in new code (examples/control_flow.ry unchanged — pre-existing)
- [x] ?? nullish coalescing operator: No new implementation in diffs (existing token.rs/tests are pre-existing)
- [x] Tagged templates: No implementation (only mentioned in docs/spec.md — documentation)
- [x] Match guards (if in match arms): Not found in codegen
- [x] BigInt operators (+,-,*,/ on BigInt): Not found in new code
- [x] Async combinators (spawn_all, join_all, futures::): Not found in new code

Scope Contamination / Bonus Features:
1. T4 MISSING — Template literal codegen (Must Have P1) is completely absent from the working tree.
   - Expr::TemplateLiteral exists in parser/ast.rs and typechecker/inference.rs
   - No handler in codegen/expr.rs or any other codegen file
   - This is a spec compliance failure, not scope creep.

2. typechecker/patterns.rs exhaustiveness improvements — Minor bonus work.
   - Improved pattern_covered_cases and find_missing_cases for bool/string/int/float literals.
   - Not explicitly scoped in plan, but supports T11 match statement type-checking.
   - Verdict: Minor adjacent-code fix; acceptable.

3. decl.rs rustfmt formatting — Minor refactoring of existing working code.
   - Only whitespace/line-break changes, no functional changes.
   - Verdict: Very minor "don't refactor" violation; no functional impact.

4. exception.rs `#[repr(C)]` on StackFrame — Minor ABI annotation.
   - Likely required for T7 Itanium C++ ABI compatibility.
   - Verdict: Acceptable as part of T7.

5. ruyi_iter_next declared in codegen/builtins.rs but unused — Dead code.
   - for_of codegen uses inline AST-based `.iter()` / `.next()` calls instead.
   - Verdict: Minor dead code, not a feature.

VERDICT: REJECT

Reason: T4 (Template literal codegen) is a Must Have P1 feature that was not implemented.
All other 12 tasks are present and compliant. No v0.5+ scope creep detected.

---

SCOPE FIDELITY CHECK — RE-RUN (After T4 Fix)
===============================================
Date: 2026-05-04
Branch: dev/v0.4.1
Base: dev/v0.4

TASKS COMPLIANT: 13/13

  T0  Version bump to 0.4.1                          DONE (Cargo.toml:9, main.rs:18)
  T1  for/for-in/for-of loop codegen                 DONE (stmt.rs: compile_for, compile_for_in, compile_for_of)
  T2  break/continue codegen                         DONE (stmt.rs: Statement::Break/Continue with loop_stack)
  T3  Optional chaining ?. + computed member         DONE (expr.rs: compile_optional_member_access, MemberProperty::Expr)
  T4  Template literal codegen                       DONE (expr.rs: compile_template_literal lines 142-207)
  T5  BigInt literal codegen                         DONE (expr.rs: compile_bigint_literal)
  T6  Audit try/catch codegen                        DONE (TRY_CATCH_AUDIT.md exists)
  T7  Exception landing pad wiring                   DONE (runtime.rs: _Unwind_RaiseException, __cxa_begin/end_catch; stmt.rs: build_invoke)
  T8  async/await true async                         DONE (async_runtime.rs: ruyi_await uses scheduler suspend_current)
  T9  async GC roots for GenerationalCollector        DONE (generational.rs: register_async_roots)
  T10 impl Trait for built-in types                  DONE (traits.rs: TypeAnnotation::Builtin)
  T11 match statement codegen                        DONE (patterns.rs: compile_match_stmt with bool/string/nullable dispatch; stmt.rs: Statement::Match)
  T12 Thread-local GC heaps                          DONE (gc_exports.rs: thread_local CURRENT_COLLECTOR with GenerationalCollector)

Must NOT Have Violations:
- [x] Promise.all / Promise.race / Promise.any / Promise.allSettled: Only pre-existing internal Rust test helpers (test_join_all, test_race), not language features
- [x] Labeled break / labeled continue: Not found in new code
- [x] ?? nullish coalescing operator: Pre-existing tokens/tests only, no codegen implementation
- [x] Tagged templates: No implementation (only mentioned in docs/spec.md — documentation)
- [x] Match guards (if in match arms): AST field exists (pre-existing) but not used in codegen
- [x] BigInt operators (+,-,*,/ on BigInt): Not found in new code
- [x] Async combinators (spawn_all, join_all, futures::): Not found as language features
- [x] Exhaustiveness checking in codegen: Not found (typechecker responsibility)
- [x] New dependencies: No new crates added to Cargo.toml
- [x] AST/HIR structural changes: No unauthorized changes

Scope Contamination / Bonus Features:
1. T4 FIXED — Template literal codegen now present in expr.rs (lines 142-207).
   - Handles empty templates, pure string parts, and interpolated expressions
   - Uses ruyi_str_concat runtime function for string concatenation
   - This resolves the previous spec compliance failure.

2. typechecker/patterns.rs exhaustiveness improvements — Minor bonus work (pre-existing from first check).
   - Verdict: Still acceptable.

3. decl.rs rustfmt formatting — Minor refactoring (pre-existing from first check).
   - Verdict: Still acceptable.

4. exception.rs `#[repr(C)]` on StackFrame — Required for T7 (pre-existing).
   - Verdict: Still acceptable.

5. ruyi_iter_next declared in codegen/builtins.rs but unused — Dead code (pre-existing).
   - Verdict: Still acceptable.

VERDICT: APPROVE

Reason: All 13 tasks (T0-T12) are present and compliant. T4 Template literal codegen
has been successfully added to codegen/expr.rs. No v0.5+ scope creep detected.
No forbidden features added. Scope is CLEAN.
