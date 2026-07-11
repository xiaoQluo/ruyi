# Implementation Tasks: v0.5.7-p1-defects

> 12 P1 defect fixes across 4 categories (Typechecker 4 + Runtime 1 + Stdlib 4 + extra). 4 sub-batches run in parallel; sequential 3.2 → 4.9 constraint on stdlib fmt + collections Iterator methods.

## File Structure

- `Create: crates/ruyic/src/typechecker/supertraits.rs` — DFS-based supertrait cycle detection (replaces the current 2-level-only check in `traits.rs:255`)
- `Modify: crates/ruyic/src/typechecker/traits.rs` — call `validate_supertrait_cycles` from `validate_supertraits`; expose `TraitRegistry::has_cycle`
- `Modify: crates/ruyic/src/typechecker/checker.rs` — wire supertraits validation into `TypeChecker::check` error pipeline
- `Create: crates/ruyic/src/typechecker/narrowing.rs` — else-branch reverse narrowing (when `x === null` is false, narrow `x` to `T?` minus null)
- `Modify: crates/ruyic/src/typechecker/inference.rs` — call `narrowing::apply_reverse_narrow` in the false branch of `narrow_for_condition`
- `Create: crates/ruyic/src/typechecker/exhaustiveness.rs` — `Type::Union` + `Expr::Match` exhaustive match analysis module
- `Modify: crates/ruyic/src/typechecker/patterns.rs` — extend `analyze_arms` to dispatch to `exhaustiveness::check_union` when subject type is `Type::Union`
- `Create: crates/ruyic/src/typechecker/self_ty.rs` — `Self` type reference resolution at class/trait element level
- `Modify: crates/ruyic/src/typechecker/types.rs` — extend `Type::from_annotation` to recognize bare `Self` identifier in element position
- `Create: crates/ruyic/src/runtime/async_gc_roots.rs` — async-task GC root registration + suspension marker API
- `Modify: crates/ruyi_runtime/src/gc_exports.rs` — call into async root collector before `collect_full` in `ruyi_gc_collect`
- `Modify: crates/ruyi_runtime/src/async_runtime.rs` — register suspended task stacks as GC roots via `TaskSuspend`
- `Create: crates/ruyic/src/runtime/random_ffi.rs` — 5 `extern "C"` random FFI functions backed by `rand` crate
- `Modify: crates/ruyi_runtime/src/lib.rs` — `pub mod random_ffi` re-export from `ruyic`
- `Create: crates/ruyi_runtime/tests/random_ffi.rs` — runtime integration tests for random FFI
- `Create: stdlib/random.ry` — random module (random, range, shuffle, seed, choice)
- `Create: stdlib/fmt.ry` — string formatting module (format, sprintf-style specifiers)
- `Create: crates/ruyic/src/runtime/fmt_ffi.rs` — 3 `extern "C"` format FFI functions (`ruyi_fmt_format_int`, `ruyi_fmt_format_float`, `ruyi_fmt_pad_right`)
- `Modify: crates/ruyic/src/parser/ast.rs` — extend `Declaration::Function` with `annotations: Vec<String>` field
- `Modify: crates/ruyic/src/parser/parser.rs` — `parse_fn_declaration` calls `parse_annotations()` before `expect(Token::Fn)`
- `Modify: crates/ruyic/src/typechecker/checker.rs` — `TestFunctionRegistry::new` collects all `fn` declarations annotated `@test`
- `Create: crates/ruyic/src/runtime/test_registry.rs` — `TestFunctionRegistry` runtime helper + `runner` binary
- `Create: stdlib/test.ry` — test module (assert_eq, assert_ne, assert_true, assert_false, suite, run)
- `Modify: stdlib/collections.ry` — add 15 new methods to `ArrayOps` (sum, product, min, max, mean, any, all, find_index, partition, zip, unzip, take, drop, chunk, dedup) plus 5 Iterator methods (filter, take_while, skip_while, enumerate, chain)

## Interfaces

### Sub-batch 1.1 (supertraits) → Sub-batch 4.2 (collections)

- **Produces**: `TraitRegistry::has_cycle(trait_name: &str) -> bool` in `crates/ruyic/src/typechecker/supertraits.rs` — full DFS detection across arbitrary depth
- **Consumed by**: Sub-batch 4.2 collections extension. The new `sum` / `product` / `min` / `max` methods require `trait Add` (in `stdlib/collections.ry`) to compile only when the registry reports no supertrait cycles for `Add`
- **Produces**: `DiagnosticKind::SupertraitCycle { chain: Vec<String> }` — emitted with the full DFS path (e.g. `["A", "B", "C", "A"]`), unlike the current message which only reports 2 levels

### Sub-batch 1.2 (narrowing) → Sub-batch 4.2 (collections partition/find_index)

- **Produces**: `narrowing::apply_reverse_narrow(env: &mut TypeEnvironment, name: &str, original_ty: &Type)` in `crates/ruyic/src/typechecker/narrowing.rs`
- **Consumed by**: Sub-batch 4.2 collections extension. The `partition` / `find_index` methods use narrowing inside `match` arms on union-typed elements; reverse narrowing ensures the `else` arm sees the correctly-widened type

### Sub-batch 1.3 (exhaustiveness) → All stdlib match-using modules

- **Produces**: `exhaustiveness::check_union(union_ty: &Type, arms: &[MatchArm]) -> ExhaustivenessReport` in `crates/ruyic/src/typechecker/exhaustiveness.rs`
- **Consumed by**: any stdlib match on `Type::Union` (e.g., `Option<T>::Some | None` matching in `stdlib/option.ry`, the new `assert_eq` in `stdlib/test.ry`, the new `partition` in `stdlib/collections.ry`)
- **Reports**: `is_exhaustive: bool`, `missing_cases: Vec<Type>`, `redundant_arms: Vec<usize>` — same struct shape as `crates/ruyic/src/typechecker/patterns.rs::PatternAnalysis`

### Sub-batch 1.4 (Self type) → Sub-batch 4.2 (collections)

- **Produces**: `self_ty::resolve(element_ctx: &ElementContext) -> Type` in `crates/ruyic/src/typechecker/self_ty.rs`
- **Consumed by**: Sub-batch 4.2 collections. Self-referential collection methods like `Array<T>::zip(self, other: Array<U>): Array<(T, U)>` need Self-aware return-type inference for the new methods that return the same collection type

### Sub-batch 2.1 (async GC roots) → Sub-batch 4.1 (test registry)

- **Produces**: `async_gc_roots::register_suspended_task(task_id: u64, stack_base: *mut u8)` and `async_gc_roots::unregister_suspended_task(task_id: u64)` in `crates/ruyic/src/runtime/async_gc_roots.rs`
- **Consumed by**: Sub-batch 4.1 test runner. Async `@test fn` suspends during test execution; test runner must register/unregister suspended task stacks as GC roots so partially-evaluated test fixtures survive `ruyi_gc_collect`

### Sub-batch 3.2 (fmt) → Sub-batch 4.2 collections methods 4.9

- **Produces**: `fmt::format(value: Any, spec: string) -> string` in `stdlib/fmt.ry` (exposed via FFI `ruyi_fmt_format_int` / `ruyi_fmt_format_float` / `ruyi_fmt_pad_right`)
- **Consumed by**: Sub-batch 4.2 collections method 4.9 (Iterator `filter` / `partition` error messages). Collections methods 4.6–4.8 can be implemented without fmt, but method 4.9 requires fmt for readable error output
- **Sequential constraint**: Sub-batch 3.2 must complete (test passes) before Sub-batch 4.2 task 4.9 begins

### Sub-batch 1.1 → Sub-batch 4.2 (BLOCKED — collections cannot start until supertraits cycle detection is merged)

- All 5 Sub-batch 4.2 tasks (4.6–4.10) `Depends on: Sub-batch 1.1` because the new `trait Add` declaration in `stdlib/collections.ry` must pass cycle validation before any ArrayOps method can be compiled

## 1. Batch 1: Typechecker (4 features, 12 tasks)

### Sub-batch 1.1: Supertrait cycle detection (full DFS)

- [ ] **1.1 编写失败的测试**

```rust
// crates/ruyic/tests/supertraits_cycle.rs
use ruyic::parser::Parser;
use ruyic::typechecker::checker::TypeChecker;
use ruyic::typechecker::traits::TraitRegistry;

#[test]
fn three_level_cycle_is_detected() {
    let source = r#"
        trait A extends B {}
        trait B extends C {}
        trait C extends A {}
    "#;
    let mut parser = Parser::new(source).expect("lexer");
    let program = parser.parse().expect("parse");
    let registry = TraitRegistry::build_from_program(&program);
    assert!(registry.has_cycle("A"), "A->B->C->A must be flagged");
    assert!(registry.has_cycle("B"), "B->C->A->B must be flagged");
    assert!(registry.has_cycle("C"), "C->A->B->C must be flagged");
}

#[test]
fn linear_supertrait_chain_passes() {
    let source = r#"
        trait A {}
        trait B extends A {}
        trait C extends B {}
    "#;
    let mut parser = Parser::new(source).expect("lexer");
    let program = parser.parse().expect("parse");
    let registry = TraitRegistry::build_from_program(&program);
    assert!(!registry.has_cycle("A"));
    assert!(!registry.has_cycle("B"));
    assert!(!registry.has_cycle("C"));
}

#[test]
fn two_level_cycle_still_detected_after_refactor() {
    let source = r#"
        trait X extends Y {}
        trait Y extends X {}
    "#;
    let mut parser = Parser::new(source).expect("lexer");
    let program = parser.parse().expect("parse");
    let registry = TraitRegistry::build_from_program(&program);
    assert!(registry.has_cycle("X"));
    assert!(registry.has_cycle("Y"));
}

#[test]
fn unknown_supertrait_still_reported() {
    let source = r#"
        trait A extends Missing {}
    "#;
    let mut parser = Parser::new(source).expect("lexer");
    let program = parser.parse().expect("parse");
    let mut checker = TypeChecker::new();
    let result = checker.check(&program);
    let messages: Vec<String> = result.diagnostics.iter()
        .map(|d| d.message.clone())
        .collect();
    assert!(
        messages.iter().any(|m| m.contains("unknown trait")),
        "expected 'unknown trait' diagnostic, got: {:?}", messages
    );
}
```

**Files**: `Create: crates/ruyic/tests/supertraits_cycle.rs`

- [ ] **1.2 运行测试并确认失败**

Run: `cargo test -p ruyic --test supertraits_cycle -- --nocapture`
Expected: FAIL — compilation error `cannot find type TraitRegistry` or `has_cycle is not a method`. Current `validate_supertraits` (traits.rs:255) only checks immediate 2-level cycles via `super_info.supertraits.contains(name)`, so the 3-level case is not detected.

- [ ] **1.3 实现最小化代码**

```rust
// crates/ruyic/src/typechecker/supertraits.rs
//! DFS-based supertrait cycle detection across arbitrary depth.
/**
 * @author Ruyi Team
 * @date 2026-07-11
 */
use std::collections::HashSet;
use crate::typechecker::diagnostics::{DiagnosticBag, DiagnosticKind};
use crate::typechecker::traits::TraitRegistry;

impl TraitRegistry {
    /// Returns true if `start` participates in any supertrait cycle
    /// (including transitive closure of arbitrary depth).
    pub fn has_cycle(&self, start: &str) -> bool {
        let mut visited: HashSet<String> = HashSet::new();
        let mut stack: HashSet<String> = HashSet::new();
        self.dfs_has_cycle(start, &mut visited, &mut stack)
    }

    fn dfs_has_cycle(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
    ) -> bool {
        if stack.contains(node) {
            return true;
        }
        if visited.contains(node) {
            return false;
        }
        visited.insert(node.to_string());
        stack.insert(node.to_string());
        if let Some(info) = self.get_trait(node) {
            for sup in &info.supertraits {
                if self.dfs_has_cycle(sup, visited, stack) {
                    return true;
                }
            }
        }
        stack.remove(node);
        false
    }

    /// Validates supertrait hierarchies for cycles of any depth.
    /// Emits `DiagnosticKind::SupertraitCycle` with the full DFS chain.
    pub fn validate_supertrait_cycles(&self, diagnostics: &mut DiagnosticBag) {
        for (name, _) in self.iter_traits() {
            if self.has_cycle(name) {
                let chain = self.collect_cycle_chain(name);
                diagnostics.add_error(DiagnosticKind::SupertraitCycle { chain });
            }
        }
    }

    fn collect_cycle_chain(&self, start: &str) -> Vec<String> {
        let mut chain = vec![start.to_string()];
        let mut current = start.to_string();
        let mut seen: HashSet<String> = HashSet::new();
        seen.insert(current.clone());
        for _ in 0..64 {
            let next_opt = self.get_trait(&current)
                .and_then(|info| info.supertraits.first().cloned());
            match next_opt {
                Some(next) if !seen.contains(&next) => {
                    seen.insert(next.clone());
                    chain.push(next.clone());
                    current = next;
                    if current == start {
                        break;
                    }
                }
                _ => break,
            }
        }
        chain
    }
}
```

```rust
// Append to crates/ruyic/src/typechecker/diagnostics.rs DiagnosticKind enum:
// (alongside existing variants)
#[error("Supertrait cycle detected: {}", .chain.join(" -> "))]
SupertraitCycle { chain: Vec<String> },
```

```rust
// Modify crates/ruyic/src/typechecker/traits.rs validate_supertraits():
// Replace the body with:
pub fn validate_supertraits(&self, diagnostics: &mut DiagnosticBag) {
    for (name, info) in &self.traits {
        for super_name in &info.supertraits {
            if !self.traits.contains_key(super_name) {
                diagnostics.add_error(DiagnosticKind::Other {
                    message: format!("Trait '{}' extends unknown trait '{}'", name, super_name),
                });
            }
        }
    }
    self.validate_supertrait_cycles(diagnostics);
}
```

```rust
// Modify crates/ruyic/src/typechecker/checker.rs TypeChecker::check():
// After line 83 (registry.validate_supertraits), the existing call already
// runs; add cycle re-validation as a redundant safety net:
registry.validate_supertrait_cycles(&mut trait_diagnostics);
```

**Files**: `Create: crates/ruyic/src/typechecker/supertraits.rs`, `Modify: crates/ruyic/src/typechecker/diagnostics.rs`, `Modify: crates/ruyic/src/typechecker/traits.rs`, `Modify: crates/ruyic/src/typechecker/checker.rs`

- [ ] **1.4 运行测试并确认通过**

Run: `cargo test -p ruyic --test supertraits_cycle -- --nocapture`
Expected: PASS — all 4 tests pass. The 3-level cycle is detected, 2-level still works, linear chain passes, unknown supertrait still produces the existing diagnostic.

- [ ] **1.5 提交**

```bash
git add crates/ruyic/src/typechecker/supertraits.rs \
        crates/ruyic/src/typechecker/diagnostics.rs \
        crates/ruyic/src/typechecker/traits.rs \
        crates/ruyic/src/typechecker/checker.rs \
        crates/ruyic/tests/supertraits_cycle.rs
git commit -m "feat(typechecker): DFS-based supertrait cycle detection (any depth)"
```

### Sub-batch 1.2: Narrowing — else-branch reverse narrowing

- [ ] **1.6 编写失败的测试**

```rust
// crates/ruyic/tests/narrowing_reverse.rs
use ruyic::parser::Parser;
use ruyic::typechecker::checker::TypeChecker;

#[test]
fn else_branch_widens_after_strict_null_check() {
    // In the `else` branch of `if (x !== null)`, x must be narrowed to null.
    let source = r#"
        fn check(x: int?): int {
            if (x === null) {
                return 0;
            } else {
                return x + 1;
            }
        }
    "#;
    let mut parser = Parser::new(source).expect("lexer");
    let program = parser.parse().expect("parse");
    let mut checker = TypeChecker::new();
    let result = checker.check(&program);
    assert!(!result.has_errors, "errors: {:?}", result.diagnostics);
}

#[test]
fn else_branch_widens_after_instanceof_check() {
    let source = r#"
        class A { x: int; }
        class B extends A { y: int; }
        fn use_a(obj: A): int {
            if (obj instanceof B) {
                return 0;
            } else {
                return obj.x;
            }
        }
    "#;
    let mut parser = Parser::new(source).expect("lexer");
    let program = parser.parse().expect("parse");
    let mut checker = TypeChecker::new();
    let result = checker.check(&program);
    assert!(!result.has_errors, "errors: {:?}", result.diagnostics);
}
```

**Files**: `Create: crates/ruyic/tests/narrowing_reverse.rs`

- [ ] **1.7 运行测试并确认失败**

Run: `cargo test -p ruyic --test narrowing_reverse -- --nocapture`
Expected: FAIL — second test fails with "obj.x not found" because `obj` is still narrowed to `B` in the else branch. The current `narrow_for_condition` (inference.rs:1640) only narrows in `true_branch=true` cases; the false branch (`true_branch=false`) leaves the variable unchanged, so the `else` body sees `x` as still nullable when it should be widened to `int`.

- [ ] **1.8 实现最小化代码**

```rust
// crates/ruyic/src/typechecker/narrowing.rs
//! Else-branch reverse narrowing: when a strict-null or instanceof check
//! is false, the variable is widened to exclude the narrowed type.
/**
 * @author Ruyi Team
 * @date 2026-07-11
 */
use crate::typechecker::environment::TypeEnvironment;
use crate::typechecker::types::Type;

pub fn apply_reverse_narrow(
    env: &mut TypeEnvironment,
    name: &str,
    original_ty: &Type,
    narrowed_in_true_branch: &Type,
) {
    // Original = T | null, narrowed = T -> else sees null only
    if let Type::Nullable(inner) = original_ty {
        if inner.as_ref() == narrowed_in_true_branch {
            env.narrow(name, Type::Null);
            return;
        }
    }
    // Original = union, narrowed = A -> else sees union without A
    if let Type::Union(variants) = original_ty {
        let remaining: Vec<Type> = variants
            .iter()
            .filter(|v| *v != narrowed_in_true_branch)
            .cloned()
            .collect();
        if remaining.len() == 1 {
            env.narrow(name, remaining.into_iter().next().unwrap());
        } else if remaining.len() > 1 {
            env.narrow(name, Type::Union(remaining));
        }
    }
}
```

```rust
// Modify crates/ruyic/src/typechecker/inference.rs narrow_for_condition()
// In the StrictEquals / StrictNotEquals arms, when true_branch is false,
// call apply_reverse_narrow. Example patch for the null case:
//   } else { // true_branch == false
//       if let Expr::NullLiteral = right.as_ref() {
//           if let Expr::Identifier(name) = left.as_ref() {
//               if let Some(orig) = self.env.lookup(name).cloned() {
//                   narrowing::apply_reverse_narrow(
//                       &mut self.env, name, &orig, &Type::Null);
//               }
//           }
//       }
//   }
```

**Files**: `Create: crates/ruyic/src/typechecker/narrowing.rs`, `Modify: crates/ruyic/src/typechecker/inference.rs`

- [ ] **1.9 运行测试并确认通过**

Run: `cargo test -p ruyic --test narrowing_reverse -- --nocapture`
Expected: PASS — both tests pass. `else` branch of `if (x === null)` sees `x: int`, else branch of `if (obj instanceof B)` sees `obj: A`.

- [ ] **1.10 提交**

```bash
git add crates/ruyic/src/typechecker/narrowing.rs \
        crates/ruyic/src/typechecker/inference.rs \
        crates/ruyic/tests/narrowing_reverse.rs
git commit -m "feat(typechecker): reverse narrowing in else branch for null/instanceof checks"
```

### Sub-batch 1.3: Exhaustiveness for Type::Union and Expr::Match

- [ ] **1.11 编写失败的测试**

```rust
// crates/ruyic/tests/exhaustiveness_union.rs
use ruyic::parser::Parser;
use ruyic::typechecker::checker::TypeChecker;

#[test]
fn match_on_union_with_all_variants_is_exhaustive() {
    let source = r#"
        type Result = Ok | Err;
        class Ok { v: int; }
        class Err { e: string; }
        fn unwrap(r: Result): int {
            match (r) {
                Ok(v) => { return v; }
                Err(e) => { return 0; }
            }
        }
    "#;
    let mut parser = Parser::new(source).expect("lexer");
    let program = parser.parse().expect("parse");
    let mut checker = TypeChecker::new();
    let result = checker.check(&program);
    assert!(!result.has_errors, "errors: {:?}", result.diagnostics);
}

#[test]
fn match_on_union_missing_variant_emits_diagnostic() {
    let source = r#"
        type Result = Ok | Err;
        class Ok { v: int; }
        class Err { e: string; }
        fn unwrap(r: Result): int {
            match (r) {
                Ok(v) => { return v; }
            }
        }
    "#;
    let mut parser = Parser::new(source).expect("lexer");
    let program = parser.parse().expect("parse");
    let mut checker = TypeChecker::new();
    let result = checker.check(&program);
    let messages: Vec<String> = result.diagnostics.iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect();
    assert!(
        messages.iter().any(|m| m.contains("non-exhaustive") || m.contains("missing")),
        "expected non-exhaustive diagnostic, got: {:?}", messages
    );
}
```

**Files**: `Create: crates/ruyic/tests/exhaustiveness_union.rs`

- [ ] **1.12 运行测试并确认失败**

Run: `cargo test -p ruyic --test exhaustiveness_union -- --nocapture`
Expected: FAIL — second test passes vacuously today (no exhaustiveness check exists for Type::Union). After implementing `check_union`, the second test must produce a diagnostic, and after fixing the test to expect the diagnostic, both tests pass.

- [ ] **1.13 实现最小化代码**

```rust
// crates/ruyic/src/typechecker/exhaustiveness.rs
//! Exhaustiveness analysis for `match` expressions over `Type::Union`.
/**
 * @author Ruyi Team
 * @date 2026-07-11
 */
use std::collections::HashSet;
use crate::parser::ast::{MatchArm, Expr};
use crate::typechecker::types::Type;

#[derive(Debug, Clone, PartialEq)]
pub struct ExhaustivenessReport {
    pub is_exhaustive: bool,
    pub missing_cases: Vec<Type>,
    pub redundant_arms: Vec<usize>,
}

pub fn check_union(union_ty: &Type, arms: &[MatchArm]) -> ExhaustivenessReport {
    let variants = match union_ty {
        Type::Union(v) => v.clone(),
        _ => return ExhaustivenessReport {
            is_exhaustive: true,
            missing_cases: vec![],
            redundant_arms: vec![],
        },
    };
    let mut covered: HashSet<String> = HashSet::new();
    let mut redundant = vec![];
    for (idx, arm) in arms.iter().enumerate() {
        let key = arm_variant_key(&arm.pattern);
        if key.is_none() {
            // Wildcard or binding pattern — covers the rest.
            covered = variants.iter().map(type_key).collect();
            continue;
        }
        if !covered.insert(key.clone().unwrap()) {
            redundant.push(idx);
        }
    }
    let missing: Vec<Type> = variants
        .into_iter()
        .filter(|v| !covered.contains(&type_key(v)))
        .collect();
    ExhaustivenessReport {
        is_exhaustive: missing.is_empty(),
        missing_cases: missing,
        redundant_arms: redundant,
    }
}

fn type_key(t: &Type) -> String {
    match t {
        Type::Named(name, _) => name.clone(),
        Type::Generic { base, .. } => base.clone(),
        other => format!("{:?}", other),
    }
}

fn arm_variant_key(pat: &crate::parser::ast::Pattern) -> Option<String> {
    use crate::parser::ast::Pattern;
    match pat {
        Pattern::Class { name, .. } => Some(name.clone()),
        Pattern::Tuple(_)
        | Pattern::Array(_)
        | Pattern::Object(_)
        | Pattern::Literal(_)
        | Pattern::Wildcard
        | Pattern::Binding(_) => None,
    }
}
```

```rust
// Modify crates/ruyic/src/typechecker/patterns.rs analyze_arms():
// At the top of the function, after computing the subject type:
let report = if let Type::Union(_) = subject_type {
    crate::typechecker::exhaustiveness::check_union(subject_type, arms)
} else {
    // existing exhaustive analysis for bool / null / named types
    analyze_arms_legacy(arms)
};
// Return report; merge missing_cases diagnostics into the bag.
```

**Files**: `Create: crates/ruyic/src/typechecker/exhaustiveness.rs`, `Modify: crates/ruyic/src/typechecker/patterns.rs`

- [ ] **1.14 运行测试并确认通过**

Run: `cargo test -p ruyic --test exhaustiveness_union -- --nocapture`
Expected: PASS — `match (r) { Ok(v) => ..., Err(e) => ... }` is exhaustive; `match (r) { Ok(v) => ... }` triggers the non-exhaustive diagnostic.

- [ ] **1.15 提交**

```bash
git add crates/ruyic/src/typechecker/exhaustiveness.rs \
        crates/ruyic/src/typechecker/patterns.rs \
        crates/ruyic/tests/exhaustiveness_union.rs
git commit -m "feat(typechecker): exhaustive match analysis for Type::Union subjects"
```

### Sub-batch 1.4: Self-referential — element-level Self reference

- [ ] **1.16 编写失败的测试**

```rust
// crates/ruyic/tests/self_referential.rs
use ruyic::parser::Parser;
use ruyic::typechecker::checker::TypeChecker;

#[test]
fn self_type_in_class_element_signature_resolves_to_class() {
    // `Self` in an element (method) signature must resolve to the enclosing class.
    let source = r#"
        class Node {
            value: int;
            fn successor(self): Node { return self; }
        }
    "#;
    let mut parser = Parser::new(source).expect("lexer");
    let program = parser.parse().expect("parse");
    let mut checker = TypeChecker::new();
    let result = checker.check(&program);
    assert!(!result.has_errors, "errors: {:?}", result.diagnostics);
}

#[test]
fn self_type_in_trait_method_signature_resolves_to_trait() {
    let source = r#"
        trait Chainable {
            fn then(self): Self;
        }
        class Node {
            fn then(self): Node { return self; }
        }
    "#;
    let mut parser = Parser::new(source).expect("lexer");
    let program = parser.parse().expect("parse");
    let mut checker = TypeChecker::new();
    let result = checker.check(&program);
    assert!(!result.has_errors, "errors: {:?}", result.diagnostics);
}
```

**Files**: `Create: crates/ruyic/tests/self_referential.rs`

- [ ] **1.17 运行测试并确认失败**

Run: `cargo test -p ruyic --test self_referential -- --nocapture`
Expected: FAIL — `Self` is not recognized in element (method parameter or return) position; current `Type::from_annotation` in `types.rs` treats bare `Self` as an unresolved identifier and emits "unknown type Self" diagnostic.

- [ ] **1.18 实现最小化代码**

```rust
// crates/ruyic/src/typechecker/self_ty.rs
//! Resolution of the bare `Self` type identifier at element level.
/**
 * @author Ruyi Team
 * @date 2026-07-11
 */
use crate::parser::ast::TypeAnnotation;
use crate::typechecker::types::Type;

pub struct ElementContext<'a> {
    pub enclosing_class: Option<&'a str>,
    pub enclosing_trait: Option<&'a str>,
}

pub fn resolve(ann: &TypeAnnotation, ctx: &ElementContext) -> Option<Type> {
    if let TypeAnnotation::Identifier(name) = ann {
        if name == "Self" {
            if let Some(c) = ctx.enclosing_class {
                return Some(Type::Named(c.to_string(), vec![]));
            }
            if let Some(t) = ctx.enclosing_trait {
                return Some(Type::Trait(t.to_string()));
            }
        }
    }
    None
}
```

```rust
// Modify crates/ruyic/src/typechecker/types.rs Type::from_annotation():
// Add this arm before the fallback identifier handling:
TypeAnnotation::Identifier(name) if name == "Self" => {
    // Defer resolution to element-level resolver; emit Type::Error for now
    // so the per-element resolver can fix it up.
    Type::Error
}
```

```rust
// Modify crates/ruyic/src/typechecker/inference.rs synthesize_element():
// After parsing the element signature, call:
if let Some(ty) = self_ty::resolve(
    &element.return_type.clone().unwrap(),
    &ElementContext {
        enclosing_class: Some(&class_name),
        enclosing_trait: None,
    },
) {
    self.env.bind("__self_resolved", ty);
}
```

**Files**: `Create: crates/ruyic/src/typechecker/self_ty.rs`, `Modify: crates/ruyic/src/typechecker/types.rs`, `Modify: crates/ruyic/src/typechecker/inference.rs`

- [ ] **1.19 运行测试并确认通过**

Run: `cargo test -p ruyic --test self_referential -- --nocapture`
Expected: PASS — both `class Node { fn successor(self): Node ... }` and `trait Chainable { fn then(self): Self; }` resolve `Self` correctly.

- [ ] **1.20 提交**

```bash
git add crates/ruyic/src/typechecker/self_ty.rs \
        crates/ruyic/src/typechecker/types.rs \
        crates/ruyic/src/typechecker/inference.rs \
        crates/ruyic/tests/self_referential.rs
git commit -m "feat(typechecker): resolve bare Self identifier at element level"
```

## 2. Batch 2: Runtime (1 feature, 3 tasks)

### Sub-batch 2.1: Async GC roots

- [ ] **2.1 编写失败的测试**

```rust
// crates/ruyi_runtime/tests/async_gc_roots.rs
use ruyi_runtime::async_runtime::{spawn_suspended, resume_task, TaskId};
use ruyi_runtime::gc::{gc_collect, gc_alloc};

#[test]
fn suspended_task_payload_survives_gc_collect() {
    // Allocate a GC object, capture it in a suspended task, trigger GC,
    // resume the task, and verify the payload is still valid.
    let payload = gc_alloc(64);
    let task_id: TaskId = spawn_suspended(Box::new(move || {
        // Closure receives payload via the captured environment.
        unsafe { *(payload as *const u32) };
    }));
    gc_collect();
    // After gc_collect, the payload must still be alive because the
    // suspended task registers it as a root.
    resume_task(task_id);
    unsafe {
        assert_eq!(*(payload as *const u32), 0, "payload must survive GC");
    }
}
```

**Files**: `Create: crates/ruyi_runtime/tests/async_gc_roots.rs`

- [ ] **2.2 运行测试并确认失败**

Run: `cargo test -p ruyi_runtime --test async_gc_roots -- --nocapture`
Expected: FAIL — currently `ruyi_gc_collect` (gc_exports.rs:39) calls `collect_full` without consulting suspended-task stacks, so the payload is collected and the assertion fails. Comment at `async_runtime.rs:429` already acknowledges this gap.

- [ ] **2.3 实现最小化代码**

```rust
// crates/ruyic/src/runtime/async_gc_roots.rs
//! GC root registration for suspended async tasks.
/**
 * @author Ruyi Team
 * @date 2026-07-11
 */
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
pub struct AsyncGcRoots {
    inner: Mutex<HashMap<u64, Vec<*mut u8>>>,
}

impl AsyncGcRoots {
    pub fn new() -> Self { Self::default() }

    pub fn register(&self, task_id: u64, stack_base: *mut u8) {
        self.inner.lock().unwrap()
            .entry(task_id).or_default().push(stack_base);
    }

    pub fn unregister(&self, task_id: u64) {
        self.inner.lock().unwrap().remove(&task_id);
    }

    /// Returns all currently registered roots (consumed by the GC mark phase).
    pub fn snapshot(&self) -> Vec<*mut u8> {
        self.inner.lock().unwrap()
            .values().flat_map(|v| v.iter().copied()).collect()
    }
}

#[no_mangle]
pub extern "C" fn ruyi_async_register_root(task_id: u64, stack_base: *mut u8) {
    crate::runtime::async_gc_roots::GLOBAL_ROOTS.register(task_id, stack_base);
}

#[no_mangle]
pub extern "C" fn ruyi_async_unregister_root(task_id: u64) {
    crate::runtime::async_gc_roots::GLOBAL_ROOTS.unregister(task_id);
}

pub static GLOBAL_ROOTS: AsyncGcRoots = AsyncGcRoots::new();
```

```rust
// Modify crates/ruyi_runtime/src/gc_exports.rs ruyi_gc_collect():
#[no_mangle]
pub extern "C" fn ruyi_gc_collect() {
    // Snapshot all async-task roots before collecting.
    let roots = ruyic::runtime::async_gc_roots::GLOBAL_ROOTS.snapshot();
    CURRENT_COLLECTOR.with(|collector| {
        let collector = collector.borrow_mut();
        for r in &roots {
            unsafe { collector.add_root(*r); }
        }
        collector.collect_full();
        for r in &roots {
            unsafe { collector.remove_root(*r); }
        }
    });
}
```

```rust
// Modify crates/ruyi_runtime/src/async_runtime.rs around line 429:
// In the task suspension path (wherever `await` parks a future), call:
extern "C" {
    fn ruyi_async_register_root(task_id: u64, stack_base: *mut u8);
    fn ruyi_async_unregister_root(task_id: u64);
}
// On suspension: ruyi_async_register_root(self.id, self.stack_base);
// On resume completion: ruyi_async_unregister_root(self.id);
```

**Files**: `Create: crates/ruyic/src/runtime/async_gc_roots.rs`, `Modify: crates/ruyi_runtime/src/gc_exports.rs`, `Modify: crates/ruyi_runtime/src/async_runtime.rs`

- [ ] **2.4 运行测试并确认通过**

Run: `cargo test -p ruyi_runtime --test async_gc_roots -- --nocapture`
Expected: PASS — payload survives `gc_collect` because the suspended task registers its stack base as a GC root.

- [ ] **2.5 提交**

```bash
git add crates/ruyic/src/runtime/async_gc_roots.rs \
        crates/ruyi_runtime/src/gc_exports.rs \
        crates/ruyi_runtime/src/async_runtime.rs \
        crates/ruyi_runtime/tests/async_gc_roots.rs
git commit -m "feat(runtime): GC roots for suspended async tasks"
```

## 3. Batch 3: Stdlib-fast (2 features, 6 tasks)

### Sub-batch 3.1: random.ry

- [ ] **3.1 编写失败的测试**

```rust
// crates/ruyi_runtime/tests/random_ffi.rs
use ruyi_runtime::random_ffi::{
    ruyi_random_seed, ruyi_random_int, ruyi_random_range,
    ruyi_random_shuffle, ruyi_random_choice,
};

#[test]
fn seed_is_deterministic() {
    ruyi_random_seed(42);
    let a = ruyi_random_int();
    ruyi_random_seed(42);
    let b = ruyi_random_int();
    assert_eq!(a, b, "same seed must produce same sequence");
}

#[test]
fn range_is_inclusive() {
    ruyi_random_seed(1);
    for _ in 0..100 {
        let v = ruyi_random_range(5, 10);
        assert!(v >= 5 && v < 10, "range out of bounds: {}", v);
    }
}

#[test]
fn choice_returns_element_from_slice() {
    let arr = vec![10, 20, 30];
    for _ in 0..50 {
        let v = ruyi_random_choice(arr.as_ptr(), arr.len());
        assert!(arr.contains(&v), "choice returned value not in slice");
    }
}
```

**Files**: `Create: crates/ruyi_runtime/tests/random_ffi.rs`

- [ ] **3.2 运行测试并确认失败**

Run: `cargo test -p ruyi_runtime --test random_ffi -- --nocapture`
Expected: FAIL — `ruyi_runtime::random_ffi` does not exist; `ruyi_random_seed`, `ruyi_random_int`, etc. are undefined.

- [ ] **3.3 实现最小化代码**

```rust
// crates/ruyic/src/runtime/random_ffi.rs
//! Random number FFI exposed to Ruyi stdlib/random.ry.
/**
 * @author Ruyi Team
 * @date 2026-07-11
 */
use std::sync::Mutex;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

static SEED: Mutex<Option<StdRng>> = Mutex::new(None);

fn with_rng<F, R>(f: F) -> R where F: FnOnce(&mut StdRng) -> R {
    let mut guard = SEED.lock().unwrap();
    if guard.is_none() {
        *guard = Some(StdRng::from_entropy());
    }
    f(guard.as_mut().unwrap())
}

#[no_mangle]
pub extern "C" fn ruyi_random_seed(seed: u64) {
    *SEED.lock().unwrap() = Some(StdRng::seed_from_u64(seed));
}

#[no_mangle]
pub extern "C" fn ruyi_random_int() -> i64 {
    with_rng(|r| r.gen())
}

#[no_mangle]
pub extern "C" fn ruyi_random_range(low: i64, high: i64) -> i64 {
    with_rng(|r| r.gen_range(low..high))
}

#[no_mangle]
pub extern "C" fn ruyi_random_choice(ptr: *const i64, len: usize) -> i64 {
    if len == 0 { return 0; }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    with_rng(|r| slice[r.gen_range(0..len)])
}

#[no_mangle]
pub extern "C" fn ruyi_random_shuffle(ptr: *mut i64, len: usize) {
    let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
    with_rng(|r| {
        for i in (1..len).rev() {
            let j = r.gen_range(0..=i);
            slice.swap(i, j);
        }
    });
}
```

```rust
// Modify crates/ruyi_runtime/src/lib.rs:
pub use ruyic_runtime_random_ffi as random_ffi; // re-export
```

```ruyi
// stdlib/random.ry
/**
 * Random number utilities for Ruyi.
 * Backed by the runtime FFI in crates/ruyic/src/runtime/random_ffi.rs.
 *
 * @author Ruyi Team
 * @date 2026-07-11
 */

extern fn __ruyi_random_seed(seed: int): void;
extern fn __ruyi_random_int(): int;
extern fn __ruyi_random_range(low: int, high: int): int;
extern fn __ruyi_random_choice(ptr: *int, len: int): int;
extern fn __ruyi_random_shuffle(ptr: *int, len: int): void;

export fn seed(s: int): void {
    __ruyi_random_seed(s);
}

export fn random(): int {
    return __ruyi_random_int();
}

export fn range(low: int, high: int): int {
    return __ruyi_random_range(low, high);
}

export fn choice<T>(items: Array<T>): T {
    return __ruyi_random_choice(items.__raw_ptr(), items.length()) as T;
}

export fn shuffle<T>(items: Array<T>): void {
    __ruyi_random_shuffle(items.__raw_ptr(), items.length());
}
```

**Files**: `Create: crates/ruyic/src/runtime/random_ffi.rs`, `Create: stdlib/random.ry`, `Modify: crates/ruyi_runtime/src/lib.rs`

- [ ] **3.4 运行测试并确认通过**

Run: `cargo test -p ruyi_runtime --test random_ffi -- --nocapture && cargo test -p ruyic --test integration -- random_smoke`
Expected: PASS — `ruyi_random_seed(42)` is deterministic, `ruyi_random_range(5, 10)` returns values in `[5, 10)`, `ruyi_random_choice` returns elements from the slice.

- [ ] **3.5 提交**

```bash
git add crates/ruyic/src/runtime/random_ffi.rs \
        crates/ruyi_runtime/src/lib.rs \
        crates/ruyi_runtime/tests/random_ffi.rs \
        stdlib/random.ry
git commit -m "feat(stdlib): random module with 5 FFI-backed functions"
```

### Sub-batch 3.2: fmt.ry

- [ ] **3.6 编写失败的测试**

```rust
// crates/ruyi_runtime/tests/fmt_ffi.rs
use ruyi_runtime::fmt_ffi::{
    ruyi_fmt_format_int, ruyi_fmt_format_float, ruyi_fmt_pad_right,
};

#[test]
fn format_int_decimal() {
    let mut buf = [0u8; 32];
    let n = unsafe {
        ruyi_fmt_format_int(42, 10, buf.as_mut_ptr(), buf.len())
    };
    let s = std::str::from_utf8(&buf[..n as usize]).unwrap();
    assert_eq!(s, "42");
}

#[test]
fn format_int_hex() {
    let mut buf = [0u8; 32];
    let n = unsafe {
        ruyi_fmt_format_int(255, 16, buf.as_mut_ptr(), buf.len())
    };
    let s = std::str::from_utf8(&buf[..n as usize]).unwrap();
    assert_eq!(s, "ff");
}

#[test]
fn pad_right_aligns() {
    let mut buf = [0u8; 32];
    let n = unsafe {
        ruyi_fmt_pad_right(b"hi".as_ptr(), 2, 5, buf.as_mut_ptr(), buf.len())
    };
    let s = std::str::from_utf8(&buf[..n as usize]).unwrap();
    assert_eq!(s, "hi   ");
}
```

**Files**: `Create: crates/ruyi_runtime/tests/fmt_ffi.rs`

- [ ] **3.7 运行测试并确认失败**

Run: `cargo test -p ruyi_runtime --test fmt_ffi -- --nocapture`
Expected: FAIL — `ruyi_runtime::fmt_ffi` does not exist; `ruyi_fmt_format_int`, `ruyi_fmt_format_float`, `ruyi_fmt_pad_right` are undefined.

- [ ] **3.8 实现最小化代码**

```rust
// crates/ruyic/src/runtime/fmt_ffi.rs
//! Formatting FFI exposed to Ruyi stdlib/fmt.ry.
/**
 * @author Ruyi Team
 * @date 2026-07-11
 */

#[no_mangle]
pub extern "C" fn ruyi_fmt_format_int(
    value: i64, base: u8, out: *mut u8, out_len: usize,
) -> i64 {
    let s = match base {
        16 => format!("{:x}", value),
        8 => format!("{:o}", value),
        2 => format!("{:b}", value),
        _ => format!("{}", value),
    };
    write_bytes(out, out_len, s.as_bytes())
}

#[no_mangle]
pub extern "C" fn ruyi_fmt_format_float(
    value: f64, precision: u8, out: *mut u8, out_len: usize,
) -> i64 {
    let s = format!("{:.*}", precision as usize, value);
    write_bytes(out, out_len, s.as_bytes())
}

#[no_mangle]
pub extern "C" fn ruyi_fmt_pad_right(
    src: *const u8, src_len: usize, width: usize,
    out: *mut u8, out_len: usize,
) -> i64 {
    let pad = width.saturating_sub(src_len);
    let total = src_len + pad;
    if total > out_len { return -1; }
    unsafe {
        std::ptr::copy_nonoverlapping(src, out, src_len);
        for i in 0..pad {
            *out.add(src_len + i) = b' ';
        }
    }
    total as i64
}

fn write_bytes(out: *mut u8, out_len: usize, src: &[u8]) -> i64 {
    if src.len() > out_len { return -1; }
    unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), out, src.len()); }
    src.len() as i64
}
```

```ruyi
// stdlib/fmt.ry
/**
 * String formatting utilities for Ruyi.
 * Backed by the runtime FFI in crates/ruyic/src/runtime/fmt_ffi.rs.
 *
 * @author Ruyi Team
 * @date 2026-07-11
 */

extern fn __ruyi_fmt_format_int(value: int, base: int, out: *u8, out_len: int): int;
extern fn __ruyi_fmt_format_float(value: float, precision: int, out: *u8, out_len: int): int;
extern fn __ruyi_fmt_pad_right(src: *u8, src_len: int, width: int, out: *u8, out_len: int): int;

export fn format_int(value: int, base: int): string {
    let buf: Array<u8> = Array.withLength(32);
    let n = __ruyi_fmt_format_int(value, base, buf.__raw_ptr(), 32);
    return buf.slice(0, n).decodeUtf8();
}

export fn format_float(value: float, precision: int): string {
    let buf: Array<u8> = Array.withLength(64);
    let n = __ruyi_fmt_format_float(value, precision, buf.__raw_ptr(), 64);
    return buf.slice(0, n).decodeUtf8();
}

export fn pad_right(s: string, width: int): string {
    let src = s.encodeUtf8();
    let buf: Array<u8> = Array.withLength(width + 8);
    let n = __ruyi_fmt_pad_right(src.__raw_ptr(), src.length(), width, buf.__raw_ptr(), buf.length());
    return buf.slice(0, n).decodeUtf8();
}
```

**Files**: `Create: crates/ruyic/src/runtime/fmt_ffi.rs`, `Create: stdlib/fmt.ry`

- [ ] **3.9 运行测试并确认通过**

Run: `cargo test -p ruyi_runtime --test fmt_ffi -- --nocapture`
Expected: PASS — `format_int(42, 10)` returns `"42"`, `format_int(255, 16)` returns `"ff"`, `pad_right("hi", 5)` returns `"hi   "`.

- [ ] **3.10 提交**

```bash
git add crates/ruyic/src/runtime/fmt_ffi.rs \
        stdlib/fmt.ry \
        crates/ruyi_runtime/tests/fmt_ffi.rs
git commit -m "feat(stdlib): fmt module with int/float/pad FFI helpers"
```

## 4. Batch 4: Stdlib-heavy (2 features, 10 tasks)

Depends on: Sub-batch 1.1 (supertrait cycle detection must land first so `trait Add` compiles)

### Sub-batch 4.1: test.ry + parser @test

- [ ] **4.1 编写失败的测试**

```rust
// crates/ruyic/tests/parser_test_attr.rs
use ruyic::parser::Parser;

#[test]
fn at_test_attribute_parses_on_fn() {
    let source = r#"
        @test
        fn my_test(): void {
            assert_eq(1, 1);
        }
    "#;
    let mut parser = Parser::new(source).expect("lexer");
    let program = parser.parse().expect("parse");
    let fn_decl = program.items.iter().find_map(|item| match item {
        ruyic::parser::ast::ModuleItem::Declaration(
            ruyic::parser::ast::Declaration::Function { name, annotations, .. }
        ) => Some((name.clone(), annotations.clone())),
        _ => None,
    });
    let (name, annotations) = fn_decl.expect("fn decl present");
    assert_eq!(name, "my_test");
    assert!(annotations.contains(&"test".to_string()),
            "expected @test annotation, got {:?}", annotations);
}

#[test]
fn test_registry_collects_only_annotated_fns() {
    let source = r#"
        fn helper(): int { return 1; }
        @test
        fn test_a(): void {}
        @test
        fn test_b(): void {}
    "#;
    let mut parser = Parser::new(source).expect("lexer");
    let program = parser.parse().expect("parse");
    let registry = ruyic::runtime::test_registry::TestFunctionRegistry::new();
    registry.collect_from_program(&program);
    assert_eq!(registry.test_names(), vec!["test_a", "test_b"]);
}
```

**Files**: `Create: crates/ruyic/tests/parser_test_attr.rs`

- [ ] **4.2 运行测试并确认失败**

Run: `cargo test -p ruyic --test parser_test_attr -- --nocapture`
Expected: FAIL — `Declaration::Function` has no `annotations` field (ast.rs:32-39). The parser treats `@test` as a class annotation prefix (parser.rs:393) and routes to `parse_class_declaration`, so the fn never appears.

- [ ] **4.3 实现最小化代码**

```rust
// Modify crates/ruyic/src/parser/ast.rs Declaration::Function variant:
// Replace the existing Function variant with:
Function {
    name: String,
    type_params: Vec<TypeParam>,
    params: Vec<Param>,
    return_type: Option<TypeAnnotation>,
    body: Vec<Statement>,
    is_async: bool,
    annotations: Vec<String>,  // NEW
},
```

```rust
// Modify crates/ruyic/src/parser/parser.rs parse_fn_declaration():
fn parse_fn_declaration(&mut self) -> Result<Declaration, ParseError> {
    let annotations = self.parse_annotations(); // NEW
    self.match_token(&Token::Async);
    let is_async = self.previous_is_async;
    self.expect(Token::Fn)?;
    // ... existing body ...
    Ok(Declaration::Function {
        name,
        type_params,
        params,
        return_type,
        body,
        is_async,
        annotations,  // NEW
    })
}
```

```rust
// Modify crates/ruyic/src/parser/parser.rs parse_declaration():
// Add Fn and Async to the at-prefix routes:
Some(Token::Fn) | Some(Token::Async) | Some(Token::At) => {
    self.parse_fn_declaration()
}
```

**Files**: `Modify: crates/ruyic/src/parser/ast.rs`, `Modify: crates/ruyic/src/parser/parser.rs`

- [ ] **4.4 运行测试并确认通过（第一阶段：parser）**

Run: `cargo test -p ruyic --test parser_test_attr -- --nocapture`
Expected: PASS for the first test (`at_test_attribute_parses_on_fn`); the second test still fails because `TestFunctionRegistry` does not exist yet.

- [ ] **4.5 编写 test.ry stdlib + TestFunctionRegistry**

```rust
// crates/ruyic/src/runtime/test_registry.rs
//! Registry of @test-annotated functions collected during type checking.
/**
 * @author Ruyi Team
 * @date 2026-07-11
 */
use crate::parser::ast::{Declaration, ModuleItem, Program};

pub struct TestFunctionRegistry {
    tests: Vec<String>,
}

impl TestFunctionRegistry {
    pub fn new() -> Self { Self { tests: vec![] } }

    pub fn collect_from_program(&mut self, program: &Program) {
        for item in &program.items {
            if let ModuleItem::Declaration(Declaration::Function {
                name, annotations, ..
            }) = item {
                if annotations.iter().any(|a| a == "test") {
                    self.tests.push(name.clone());
                }
            }
        }
    }

    pub fn test_names(&self) -> &[String] { &self.tests }
}
```

```rust
// Modify crates/ruyic/src/typechecker/checker.rs TypeChecker::check():
// After trait validation, build the test registry:
let mut test_registry = crate::runtime::test_registry::TestFunctionRegistry::new();
test_registry.collect_from_program(program);
```

```ruyi
// stdlib/test.ry
/**
 * Test framework for Ruyi.
 * Provides assertion helpers and the test runner entry point.
 *
 * @author Ruyi Team
 * @date 2026-07-11
 */

export fn assert_eq<T>(actual: T, expected: T): void {
    if (actual !== expected) {
        throw "assert_eq failed: expected " + expected.toString() +
              ", got " + actual.toString();
    }
}

export fn assert_ne<T>(actual: T, expected: T): void {
    if (actual === expected) {
        throw "assert_ne failed: both values are " + actual.toString();
    }
}

export fn assert_true(cond: bool): void {
    if (!cond) { throw "assert_true failed"; }
}

export fn assert_false(cond: bool): void {
    if (cond) { throw "assert_false failed"; }
}

export fn suite(name: string, tests: Array<fn() -> void>): void {
    print("[suite] " + name);
    for (let i = 0; i < tests.length(); i = i + 1) {
        try {
            tests.get(i)();
            print("  ok");
        } catch (e) {
            print("  FAIL: " + e);
        }
    }
}
```

**Files**: `Create: crates/ruyic/src/runtime/test_registry.rs`, `Modify: crates/ruyic/src/typechecker/checker.rs`, `Create: stdlib/test.ry`

- [ ] **4.6 运行测试并确认通过（第二阶段：registry + stdlib）**

Run: `cargo test -p ruyic --test parser_test_attr -- --nocapture && cargo test -p ruyic --test integration -- test_smoke`
Expected: PASS — both parser and registry tests pass.

- [ ] **4.7 提交**

```bash
git add crates/ruyic/src/parser/ast.rs \
        crates/ruyic/src/parser/parser.rs \
        crates/ruyic/src/runtime/test_registry.rs \
        crates/ruyic/src/typechecker/checker.rs \
        crates/ruyic/tests/parser_test_attr.rs \
        stdlib/test.ry
git commit -m "feat(stdlib): test framework with @test fn attribute + registry"
```

### Sub-batch 4.2: collections.ry extension (BLOCKED on Sub-batch 1.1 supertraits)

Depends on: Sub-batch 1.1 (supertraits cycle detection must merge first so `trait Add` compiles)

- [ ] **4.8 编写失败的测试：trait Add + ArrayOps::sum**

```rust
// crates/ruyic/tests/collections_arrayops.rs
use ruyic::parser::Parser;
use ruyic::typechecker::checker::TypeChecker;

#[test]
fn array_sum_compiles_for_int() {
    let source = r#"
        trait Add<T> {
            fn add(self, other: T): T;
        }
        impl Add<int> for int {
            fn add(self, other: int): int { return self + other; }
        }
        fn total(arr: Array<int>): int {
            return arr.sum();
        }
    "#;
    let mut parser = Parser::new(source).expect("lexer");
    let program = parser.parse().expect("parse");
    let mut checker = TypeChecker::new();
    let result = checker.check(&program);
    assert!(!result.has_errors, "errors: {:?}", result.diagnostics);
}

#[test]
fn array_any_all_compile() {
    let source = r#"
        fn any_positive(arr: Array<int>): bool {
            return arr.any(fn(x: int): bool { return x > 0; });
        }
        fn all_positive(arr: Array<int>): bool {
            return arr.all(fn(x: int): bool { return x > 0; });
        }
    "#;
    let mut parser = Parser::new(source).expect("lexer");
    let program = parser.parse().expect("parse");
    let mut checker = TypeChecker::new();
    let result = checker.check(&program);
    assert!(!result.has_errors, "errors: {:?}", result.diagnostics);
}
```

**Files**: `Create: crates/ruyic/tests/collections_arrayops.rs`

- [ ] **4.9 运行测试并确认失败**

Run: `cargo test -p ruyic --test collections_arrayops -- --nocapture`
Expected: FAIL — `ArrayOps::sum`, `ArrayOps::any`, `ArrayOps::all` are not declared in `stdlib/collections.ry`. Test must fail because the methods do not exist.

- [ ] **4.10 实现 5 个 ArrayOps 新方法（sum/product/min/max/mean）+ 测试通过**

```ruyi
// Append to stdlib/collections.ry ArrayOps trait (after line 113):
/**
 * Numeric aggregation trait required for sum/product/min/max/mean.
 * @author Ruyi Team
 * @date 2026-07-11
 */
export trait Add<T> {
    fn add(self, other: T): T;
}

/**
 * Computes the sum of all elements.
 * Requires `T: Add<T>` (declared above).
 * @return Zero-value when the array is empty.
 */
fn sum(self): T;

/**
 * Computes the product of all elements.
 * @return One-value when the array is empty.
 */
fn product(self): T;

/**
 * Returns the minimum element.
 * @throws RangeError when the array is empty.
 */
fn min(self): T;

/**
 * Returns the maximum element.
 * @throws RangeError when the array is empty.
 */
fn max(self): T;

/**
 * Returns the arithmetic mean (sum / length).
 * @return 0 when the array is empty.
 */
fn mean(self): float;

/**
 * Returns true if any element satisfies the predicate.
 * @param f Predicate (T -> bool)
 */
fn any(self, f: fn(T) -> bool): bool;

/**
 * Returns true if every element satisfies the predicate.
 * @param f Predicate (T -> bool)
 */
fn all(self, f: fn(T) -> bool): bool;

/**
 * Returns the index of the first element matching the predicate, or -1.
 * @param f Predicate (T -> bool)
 */
fn find_index(self, f: fn(T) -> bool): int;

/**
 * Splits into (matching, non-matching) based on the predicate.
 * @param f Predicate (T -> bool)
 */
fn partition(self, f: fn(T) -> bool): (Array<T>, Array<T>);

/**
 * Combines two arrays element-wise into an array of pairs.
 */
fn zip<U>(self, other: Array<U>): Array<(T, U)>;

/**
 * Splits an array of pairs into a pair of arrays.
 */
fn unzip<U>(self): (Array<T>, Array<U>) where T: (T, U);

/**
 * Returns the first n elements.
 */
fn take(self, n: int): Array<T>;

/**
 * Drops the first n elements.
 */
fn drop(self, n: int): Array<T>;

/**
 * Splits into chunks of size n (last chunk may be smaller).
 */
fn chunk(self, n: int): Array<Array<T>>;

/**
 * Returns the array with consecutive duplicates removed.
 */
fn dedup(self): Array<T>;
```

```ruyi
// Append to stdlib/collections.ry ArrayOps impl block (after line ~250):
impl<T> ArrayOps<T> for Array<T> {
    fn sum(self): T {
        let acc: T = 0 as T;  // requires T: Add<T> + Zero
        let len = self.length();
        for (let i = 0; i < len; i = i + 1) {
            acc = acc.add(self.get(i));
        }
        return acc;
    }

    fn product(self): T {
        let acc: T = 1 as T;
        let len = self.length();
        for (let i = 0; i < len; i = i + 1) {
            acc = acc.mul(self.get(i));
        }
        return acc;
    }

    fn min(self): T {
        if (self.length() === 0) {
            throw RangeError.new("Cannot find min of empty array");
        }
        let best = self.get(0);
        for (let i = 1; i < self.length(); i = i + 1) {
            if (self.get(i).lt(best)) { best = self.get(i); }
        }
        return best;
    }

    fn max(self): T {
        if (self.length() === 0) {
            throw RangeError.new("Cannot find max of empty array");
        }
        let best = self.get(0);
        for (let i = 1; i < self.length(); i = i + 1) {
            if (self.get(i).gt(best)) { best = self.get(i); }
        }
        return best;
    }

    fn mean(self): float {
        if (self.length() === 0) { return 0.0; }
        return self.sum() as float / self.length() as float;
    }

    fn any(self, f: fn(T) -> bool): bool {
        for (let i = 0; i < self.length(); i = i + 1) {
            if (f(self.get(i))) { return true; }
        }
        return false;
    }

    fn all(self, f: fn(T) -> bool): bool {
        for (let i = 0; i < self.length(); i = i + 1) {
            if (!f(self.get(i))) { return false; }
        }
        return true;
    }

    fn find_index(self, f: fn(T) -> bool): int {
        for (let i = 0; i < self.length(); i = i + 1) {
            if (f(self.get(i))) { return i; }
        }
        return -1;
    }

    fn partition(self, f: fn(T) -> bool): (Array<T>, Array<T>) {
        let yes: Array<T> = [];
        let no: Array<T> = [];
        for (let i = 0; i < self.length(); i = i + 1) {
            if (f(self.get(i))) {
                yes.push(self.get(i));
            } else {
                no.push(self.get(i));
            }
        }
        return (yes, no);
    }

    fn zip<U>(self, other: Array<U>): Array<(T, U)> {
        let len = self.length();
        if (other.length() < len) { len = other.length(); }
        let result: Array<(T, U)> = [];
        for (let i = 0; i < len; i = i + 1) {
            result.push((self.get(i), other.get(i)));
        }
        return result;
    }

    fn unzip<U>(self): (Array<T>, Array<U>) {
        let ts: Array<T> = [];
        let us: Array<U> = [];
        for (let i = 0; i < self.length(); i = i + 1) {
            let pair = self.get(i);
            ts.push(pair.0);
            us.push(pair.1);
        }
        return (ts, us);
    }

    fn take(self, n: int): Array<T> {
        let cap = n;
        if (cap > self.length()) { cap = self.length(); }
        let result: Array<T> = [];
        for (let i = 0; i < cap; i = i + 1) {
            result.push(self.get(i));
        }
        return result;
    }

    fn drop(self, n: int): Array<T> {
        let start = n;
        if (start < 0) { start = 0; }
        if (start > self.length()) { start = self.length(); }
        let result: Array<T> = [];
        for (let i = start; i < self.length(); i = i + 1) {
            result.push(self.get(i));
        }
        return result;
    }

    fn chunk(self, n: int): Array<Array<T>> {
        if (n <= 0) { throw RangeError.new("chunk size must be positive"); }
        let result: Array<Array<T>> = [];
        let i = 0;
        while (i < self.length()) {
            let cap = n;
            if (i + cap > self.length()) { cap = self.length() - i; }
            result.push(self.take(i + cap).drop(i));
            i = i + n;
        }
        return result;
    }

    fn dedup(self): Array<T> {
        let result: Array<T> = [];
        for (let i = 0; i < self.length(); i = i + 1) {
            let current = self.get(i);
            if (result.find_index(fn(x: T): bool { return x === current; }) === -1) {
                result.push(current);
            }
        }
        return result;
    }
}
```

**Files**: `Modify: stdlib/collections.ry`

- [ ] **4.11 运行测试并确认通过**

Run: `cargo test -p ruyic --test collections_arrayops -- --nocapture`
Expected: PASS — `trait Add`, `ArrayOps::sum`, `ArrayOps::any`, `ArrayOps::all` all compile because Sub-batch 1.1's supertrait cycle detection prevents `Add` from accidentally forming a cycle with `Array<T>`.

- [ ] **4.12 编写 Iterator 新方法的测试**

```rust
// Append to crates/ruyic/tests/collections_arrayops.rs:
#[test]
fn iterator_filter_take_while_skip_while_enumerate_chain() {
    let source = r#"
        fn first_three_positive_squares(
            iter: Iterator<int>
        ): Array<(int, int)> {
            return iter
                .filter(fn(x: int): bool { return x > 0; })
                .take_while(fn(x: int): bool { return x <= 100; })
                .skip_while(fn(x: int): bool { return x < 10; })
                .enumerate()
                .chain(Iterator.empty())
                .collect();
        }
    "#;
    let mut parser = Parser::new(source).expect("lexer");
    let program = parser.parse().expect("parse");
    let mut checker = TypeChecker::new();
    let result = checker.check(&program);
    assert!(!result.has_errors, "errors: {:?}", result.diagnostics);
}
```

**Files**: `Modify: crates/ruyic/tests/collections_arrayops.rs`

- [ ] **4.13 实现 5 个 Iterator 新方法（filter / take_while / skip_while / enumerate / chain）+ 测试通过**

```ruyi
// Append to stdlib/collections.ry Iterator trait (after line 28):
fn filter(self, f: fn(T) -> bool): FilteredIterator<T>;
fn take_while(self, f: fn(T) -> bool): TakeWhileIterator<T>;
fn skip_while(self, f: fn(T) -> bool): SkipWhileIterator<T>;
fn enumerate(self): EnumeratedIterator<T>;
fn chain(self, other: Iterator<T>): ChainedIterator<T>;

// New iterator classes (append to stdlib/collections.ry):
class FilteredIterator<T> {
    inner: Iterator<T>;
    pred: fn(T) -> bool;
}

impl<T> Iterator<T> for FilteredIterator<T> {
    fn next(self): T? {
        while (true) {
            let v = self.inner.next();
            if (v === null) { return null; }
            if (self.pred(v as T)) { return v; }
        }
    }
}

class TakeWhileIterator<T> {
    inner: Iterator<T>;
    pred: fn(T) -> bool;
}

impl<T> Iterator<T> for TakeWhileIterator<T> {
    fn next(self): T? {
        let v = self.inner.next();
        if (v === null) { return null; }
        if (self.pred(v as T)) { return v; }
        return null;
    }
}

class SkipWhileIterator<T> {
    inner: Iterator<T>;
    pred: fn(T) -> bool;
    done_skipping: bool;
}

impl<T> Iterator<T> for SkipWhileIterator<T> {
    fn next(self): T? {
        if (!self.done_skipping) {
            while (true) {
                let v = self.inner.next();
                if (v === null) { self.done_skipping = true; return null; }
                if (!self.pred(v as T)) {
                    self.done_skipping = true;
                    return v;
                }
            }
        }
        return self.inner.next();
    }
}

class EnumeratedIterator<T> {
    inner: Iterator<T>;
    index: int;
}

impl<T> Iterator<(int, T)> for EnumeratedIterator<T> {
    fn next(self): (int, T)? {
        let v = self.inner.next();
        if (v === null) { return null; }
        let result = (self.index, v as T);
        self.index = self.index + 1;
        return result;
    }
}

class ChainedIterator<T> {
    first: Iterator<T>;
    second: Iterator<T>;
    first_done: bool;
}

impl<T> Iterator<T> for ChainedIterator<T> {
    fn next(self): T? {
        if (!self.first_done) {
            let v = self.first.next();
            if (v !== null) { return v; }
            self.first_done = true;
        }
        return self.second.next();
    }
}
```

**Files**: `Modify: stdlib/collections.ry`

- [ ] **4.14 运行测试并确认通过**

Run: `cargo test -p ruyic --test collections_arrayops -- --nocapture`
Expected: PASS — the chained `filter → take_while → skip_while → enumerate → chain → collect` pipeline type-checks.

- [ ] **4.15 提交**

```bash
git add stdlib/collections.ry \
        crates/ruyic/tests/collections_arrayops.rs
git commit -m "feat(stdlib): 15 new ArrayOps methods + 5 new Iterator combinators"
```

## 5. Closeout

- [ ] **5.1 验证所有 contract obligations**

| # | Obligation | Verification |
|---|-----------|---|
| 1 | All 12 P1 defects closed with dedicated tests | `cargo test --workspace --quiet` shows green for the new test files: `supertraits_cycle`, `narrowing_reverse`, `exhaustiveness_union`, `self_referential`, `async_gc_roots`, `random_ffi`, `fmt_ffi`, `parser_test_attr`, `collections_arrayops` |
| 2 | `cargo test --workspace` zero regressions | run `cargo test --workspace`; expect all pre-existing tests still pass |
| 3 | `cargo clippy --workspace` zero new warnings | run `cargo clippy --workspace --all-targets`; expect no new warnings |
| 4 | ≥15 new methods on Array/Iterator in collections.ry | count methods in modified `stdlib/collections.ry`: 15 ArrayOps (sum/product/min/max/mean/any/all/find_index/partition/zip/unzip/take/drop/chunk/dedup) + 5 Iterator (filter/take_while/skip_while/enumerate/chain) = 20 total |
| 5 | ≥5 new runtime FFI for random | count `#[no_mangle] pub extern "C"` in `crates/ruyic/src/runtime/random_ffi.rs`: 5 (seed/int/range/choice/shuffle) |
| 6 | Parser supports `@test` attribute on fn declarations + `TestFunctionRegistry` | `crates/ruyic/tests/parser_test_attr.rs` 2 tests pass |
| 7 | `ruyi_gc_collect` preserves GC objects reachable from suspended async tasks | `crates/ruyi_runtime/tests/async_gc_roots.rs` 1 test passes |
| 8 | `Type::Union` + `Expr::Match` exhaustive match support | `crates/ruyic/tests/exhaustiveness_union.rs` 2 tests pass |
| 9 | `docs/spec.md` and `docs/roadmap.md` updated | run `git log --oneline docs/spec.md docs/roadmap.md` shows new commits marking P1 items as done |
| 10 | `cargo fmt-check` passes | run `cargo fmt --check`; expect no diff |
| 11 | v0.5.7 release commit on `dev/v0.5.7-p1-defects` branch | `git log --oneline dev/v0.5.7-p1-defects` shows the v0.5.7 commit hash |
| 12 | Merge commit to main per AGENTS.md branch policy | `git log --oneline main | head -1` shows the merge commit from dev branch |

- [ ] **5.2 最终构建 + 测试 + lint + fmt 验证**

```bash
make check
make build-release
make test
make lint
make fmt-check
make run-example EXAMPLE=random
make run-example EXAMPLE=collections
```

Expected: All targets succeed. `make run-example EXAMPLE=random` compiles and runs `examples/random.ry` (must be created from `stdlib/random.ry` test fixtures). `make run-example EXAMPLE=collections` compiles and runs `examples/collections.ry` exercising `sum` / `partition` / `filter`.

- [ ] **5.3 总结风险、follow-ups、归档准备**

Risks:
- LLVM 14 availability on macOS: `brew install llvm@14` required. If absent, fall back to `cargo check -p ruyi_runtime --no-default-features` per AGENTS.md.
- `Self` type resolution at element level is conservative: returns `Type::Error` then fixes up via `self_ty::resolve`. Future work can fold this into `Type::from_annotation` directly.
- Async GC root snapshot is a stop-the-world pause: full GC must drain the root set before collecting. The current `collect_full` is single-threaded so this is acceptable; a future concurrent GC would need lock-free root snapshot.
- `trait Add` declaration in `stdlib/collections.ry` is a new public symbol; downstream user code that already declares `Add` will hit a name collision. Document the namespace.

Follow-ups (v0.5.8 or later):
- `Sub.batch 1.3` exhaustiveness integration tests in `crates/ruyic/tests/codegen_match.rs` (currently out of scope per DP-1)
- FFI cleanup of pre-existing undeclared `__io_*` / `__process_*` / `__path_*` symbols in `stdlib/io.ry`, `stdlib/process.ry`, `stdlib/path.ry` (out of scope per DP-1)
- Concurrent GC design (replaces stop-the-world in `collect_full`)
- Codegen.rs integration tests for the new collections methods

Archive readiness:
- All 12 P1 items closed
- All 9 test files passing
- All 4 sub-batches merged via conventional commits
- `docs/spec.md` updated with new methods
- `docs/roadmap.md` P1 section marked complete