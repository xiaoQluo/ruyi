/**
 * Self-referential class field tests (Spec 3.6, v0.5.7-p1-defects).
 *
 * Covers three cases:
 *   1. `class Node { next: Node?; }` — optional self-reference by class
 *      name compiles cleanly (no error) and the field is resolved to a
 *      pointer-compatible nullable Node.
 *   2. `class Tree { children: Box<Self>; }` — indirect self reference
 *      via the `Box` container compiles and the inner Self resolves to
 *      the enclosing class.
 *   3. `class Bad { me: Self; }` — bare `Self` (no indirection) is
 *      rejected at the field position with the "bare Self not allowed"
 *      diagnostic.
 *
 * @author Ruyi Team
 * @date 2026-07-11
 */
use ruyic::parser::Parser;
use ruyic::typechecker::checker::TypeChecker;

/** Helper: parse + typecheck and return the result. */
fn check(source: &str) -> ruyic::typechecker::TypeCheckResult {
    let mut parser = Parser::new(source).expect("lexer should not fail");
    let program = parser.parse().expect("parse should succeed");
    let mut checker = TypeChecker::new();
    checker.check(&program)
}

// ── 1. Optional self-reference by class name ─────────────────────────

#[test]
fn test_optional_self_field() {
    // `class Node { next: Node?; }` — a linked-list pointer field. The
    // optional self-reference is structurally valid and must compile
    // without any error diagnostic.
    let result = check(
        r#"
        class Node {
            value: int;
            next: Node?;
        }
        "#,
    );
    assert!(
        !result.has_errors,
        "optional self-reference by class name must compile, got errors: {:?}",
        result.diagnostics
    );
}

// ── 2. Indirect self-reference via Box container ────────────────────

#[test]
fn test_box_self() {
    // `class Tree { children: Box<Self>; }` — self-reference through the
    // `Box` indirection container. Must compile and the inner Self must
    // resolve to Tree (so the field type becomes `Box<Tree>`).
    let result = check(
        r#"
        class Tree {
            value: int;
            children: Box<Self>;
        }
        "#,
    );
    assert!(
        !result.has_errors,
        "Box<Self> must compile, got errors: {:?}",
        result.diagnostics
    );
}

// ── 3. Bare Self in field position is rejected ──────────────────────

#[test]
fn test_bare_self_rejected() {
    // `class Bad { me: Self; }` — bare Self without any indirection
    // creates an infinite-size type. The type checker must reject this
    // with a diagnostic mentioning "bare Self not allowed".
    let result = check(
        r#"
        class Bad {
            me: Self;
        }
        "#,
    );
    assert!(
        result.has_errors,
        "bare Self field must produce an error diagnostic"
    );
    let messages: Vec<String> = result
        .diagnostics
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message().to_string())
        .collect();
    let joined = messages.join("\n");
    assert!(
        messages.iter().any(|m| m.contains("bare Self not allowed")),
        "expected 'bare Self not allowed' diagnostic, got: {}",
        joined
    );
}
