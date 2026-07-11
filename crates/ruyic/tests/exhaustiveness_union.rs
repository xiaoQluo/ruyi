/**
 * Exhaustiveness checking for Type::Union in match expressions (Sub-batch 1.3).
 *
 * Verifies the `exhaustiveness::check_union` analysis:
 *   - When a `match` over `Type::Union` is missing arms, a warning
 *     diagnostic is emitted listing the missing variants.
 *   - A `_` wildcard arm suppresses the missing-arm diagnostic.
 *   - `Type::missing_arms(&self, covered)` returns the list of uncovered
 *     variant names in declaration order.
 *   - The warning does NOT promote to a hard error: type checking still
 *     succeeds (`has_errors == false`).
 *
 * @author Ruyi Team
 * @date 2026-07-11
 */
use ruyic::parser::Parser;
use ruyic::typechecker::diagnostics::DiagnosticKind;
use ruyic::typechecker::types::Type;
use ruyic::typechecker::TypeChecker;

// ── Helpers ───────────────────────────────────────────────────

fn parse_program(source: &str) -> ruyic::parser::ast::Program {
    let mut parser = Parser::new(source).expect("lexer should not fail");
    parser.parse().expect("parse should succeed")
}

fn check_program(source: &str) -> ruyic::typechecker::TypeCheckResult {
    let program = parse_program(source);
    let mut checker = TypeChecker::new();
    checker.check(&program)
}

fn missing_variant_warnings(result: &ruyic::typechecker::TypeCheckResult) -> Vec<String> {
    let mut out = Vec::new();
    for d in &result.diagnostics {
        if !d.is_warning() {
            continue;
        }
        if let DiagnosticKind::NonExhaustiveMatch { missing, .. } = d.kind() {
            out.extend(missing.iter().cloned());
        }
    }
    out
}

fn diagnostic_messages(result: &ruyic::typechecker::TypeCheckResult) -> Vec<String> {
    result
        .diagnostics
        .iter()
        .map(|d| d.message().to_string())
        .collect()
}

// ── 1. Three-variant union missing one arm ─────────────────────

#[test]
fn test_three_variants_one_missing() {
    // Source: a function whose parameter is annotated as a union of three
    // named types and whose body matches only two of the three variants.
    let source = r#"
        fn describe(color: Red|Green|Blue): int {
            match (color) {
                Red => { return 1; }
                Green => { return 2; }
            }
            return 0;
        }
    "#;
    let result = check_program(source);
    assert!(
        !result.has_errors,
        "missing-arm union match must be a warning, not an error. diagnostics: {:?}",
        diagnostic_messages(&result)
    );

    let missing = missing_variant_warnings(&result);
    assert!(
        missing.iter().any(|m| m == "Blue"),
        "warning must list missing variant `Blue`, got: {:?}",
        missing
    );
    assert!(
        missing.iter().all(|m| m != "Red" && m != "Green"),
        "covered variants must not be listed as missing, got: {:?}",
        missing
    );
}

// ── 2. Wildcard arm suppresses the diagnostic ──────────────────

#[test]
fn test_wildcard_suppresses() {
    // A `_ => ...` arm already covers every remaining variant, so the
    // missing-arm diagnostic must NOT be produced.
    let source = r#"
        fn describe(color: Red|Green|Blue): int {
            match (color) {
                Red => { return 1; }
                _ => { return 0; }
            }
        }
    "#;
    let result = check_program(source);
    assert!(
        !result.has_errors,
        "wildcard match must not produce errors, diagnostics: {:?}",
        diagnostic_messages(&result)
    );

    let missing = missing_variant_warnings(&result);
    assert!(
        missing.is_empty(),
        "wildcard arm must suppress the missing-variant warning, got: {:?}",
        missing
    );
}

// ── 3. Type::missing_arms returns the uncovered variants ───────

#[test]
fn test_missing_arms_api() {
    // Direct API check: construct a `Type::Union` and verify
    // `Type::missing_arms` returns the variants not present in `covered`.
    let union_ty = Type::Union(vec![
        Type::Named("Red".to_string(), vec![]),
        Type::Named("Green".to_string(), vec![]),
        Type::Named("Blue".to_string(), vec![]),
    ]);

    let only_red = vec!["Red".to_string()];
    let missing = union_ty.missing_arms(&only_red);
    assert_eq!(
        missing,
        vec!["Green".to_string(), "Blue".to_string()],
        "missing_arms must list Green then Blue (declaration order), got: {:?}",
        missing
    );

    // Covering everything yields an empty list.
    let all: Vec<String> = vec!["Red".to_string(), "Green".to_string(), "Blue".to_string()];
    assert!(union_ty.missing_arms(&all).is_empty());

    // A non-union type returns an empty list (degenerate case).
    let int_ty = Type::Int;
    assert!(int_ty.missing_arms(&[]).is_empty());
}

// ── 4. Missing-arm match does not block downstream code ────────

#[test]
fn test_warning_not_blocking() {
    // Type checking must succeed even when a missing-arm warning is emitted.
    let source = r#"
        fn describe(color: Red|Green|Blue): int {
            match (color) {
                Red => { return 1; }
                Green => { return 2; }
            }
            return 99;
        }
    "#;
    let result = check_program(source);
    assert!(
        !result.has_errors,
        "missing-arm warning must NOT mark the program as having errors, diagnostics: {:?}",
        diagnostic_messages(&result)
    );

    // Sanity: at least one warning was emitted, so the test exercises the
    // warning path (otherwise the assertion above would be vacuous).
    assert!(
        result.diagnostics.iter().any(|d| d.is_warning()),
        "expected at least one warning to be emitted, got: {:?}",
        diagnostic_messages(&result)
    );
}

// ── 5. (Bonus) check_union called via the public API path ──────

#[test]
fn test_check_union_directly() {
    // Direct invocation of `exhaustiveness::check_union` — exercises the
    // module without going through the full inference pipeline.
    use ruyic::typechecker::diagnostics::DiagnosticBag;
    use ruyic::typechecker::exhaustiveness::{check_union, ExhaustivenessReport};

    let union_ty = Type::Union(vec![
        Type::Named("Red".to_string(), vec![]),
        Type::Named("Green".to_string(), vec![]),
        Type::Named("Blue".to_string(), vec![]),
    ]);

    // Missing one variant → warning, is_exhaustive=false, missing=[Blue].
    let mut bag = DiagnosticBag::new();
    let report = check_union(
        &mut bag,
        &union_ty,
        &["Red".to_string(), "Green".to_string()],
    );
    assert!(!report.is_exhaustive);
    assert_eq!(report.missing_cases, vec!["Blue".to_string()]);
    assert!(bag.has_warnings(), "expected a warning diagnostic");

    // Covering all variants → empty missing list, no warnings.
    let mut bag2 = DiagnosticBag::new();
    let report_full = check_union(
        &mut bag2,
        &union_ty,
        &[
            "Red".to_string(),
            "Green".to_string(),
            "Blue".to_string(),
        ],
    );
    assert!(report_full.is_exhaustive);
    assert!(report_full.missing_cases.is_empty());
    assert!(!bag2.has_warnings());

    // Wildcard suppresses the diagnostic and reports exhaustive.
    let mut bag3 = DiagnosticBag::new();
    let report_wild = check_union(
        &mut bag3,
        &union_ty,
        &["Red".to_string(), "_".to_string()],
    );
    assert!(report_wild.is_exhaustive);
    assert!(report_wild.missing_cases.is_empty());
    assert!(!bag3.has_warnings());

    // Default for non-union types.
    let mut bag4 = DiagnosticBag::new();
    let report_int: ExhaustivenessReport = check_union(&mut bag4, &Type::Int, &[]);
    assert!(report_int.is_exhaustive);
    assert!(report_int.missing_cases.is_empty());
    assert!(!bag4.has_warnings());
}