/**
 * Exhaustiveness checking for `Type::Union` and `Expr::Match`.
 *
 * Given a `match` expression whose subject type is `Type::Union`, compute
 * which variant arms are still missing. A diagnostic of severity Warning is
 * emitted for every missing variant — warnings (not errors) preserve
 * backward compatibility with code that intentionally relies on partial
 * matches (spec section 3.5, requirement 4).
 *
 * A `_` (wildcard) arm is treated as covering every remaining variant, so
 * it suppresses the missing-arm diagnostic entirely (spec section 3.5,
 * requirement 2).
 *
 * Spec ref: changes/v0.5.7-p1-defects/specs/exhaustiveness/spec.md
 *
 * @author Ruyi Team
 * @date 2026-07-11
 */
use std::collections::HashSet;

use crate::typechecker::diagnostics::{DiagnosticBag, DiagnosticKind};
use crate::typechecker::types::Type;

/// Result of an exhaustiveness check over a `Type::Union` scrutinee.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExhaustivenessReport {
    /// `true` when every variant is covered (including via a wildcard).
    pub is_exhaustive: bool,
    /// Names of variants that are not covered by any arm.
    pub missing_cases: Vec<String>,
    /// Names of arms that were subsumed by an earlier arm or wildcard.
    pub redundant_arms: Vec<String>,
}

/// Check whether `arms` covers every variant of `union_ty`.
///
/// `arms` is the ordered list of pattern identifiers as the caller observes
/// them (e.g. `["Red", "Green"]`, `["Red", "_"]`). A `_` arm is treated as
/// covering every variant and therefore suppresses the missing-arm
/// diagnostic. The function pushes a `Severity::Warning` diagnostic onto
/// `bag` listing every uncovered variant; it never raises an error so that
/// missing arms remain backward-compatible (DP-1).
pub fn check_union(
    bag: &mut DiagnosticBag,
    union_ty: &Type,
    arms: &[String],
) -> ExhaustivenessReport {
    let variants = match union_ty {
        Type::Union(vs) => vs.clone(),
        // Non-union scrutinees are out of scope for this analyser; report
        // exhaustive with no warnings so callers can use the same API for
        // any subject type.
        _ => {
            return ExhaustivenessReport {
                is_exhaustive: true,
                missing_cases: Vec::new(),
                redundant_arms: Vec::new(),
            };
        }
    };

    let variant_names: Vec<String> = variants.iter().map(variant_name).collect();
    let arm_set: HashSet<&str> = arms.iter().map(|s| s.as_str()).collect();
    let has_wildcard = arm_set.contains("_");

    let missing: Vec<String> = variant_names
        .iter()
        .filter(|n| !arm_set.contains(n.as_str()))
        .cloned()
        .collect();

    let is_exhaustive = has_wildcard || missing.is_empty();

    if !is_exhaustive {
        bag.add_warning(DiagnosticKind::NonExhaustiveMatch {
            scrutinee_type: union_ty.clone(),
            missing: missing.clone(),
        });
    }

    // A wildcard collapses the missing-variant list to empty so callers
    // see a consistent `missing_cases` alongside `is_exhaustive = true`.
    let report_missing = if has_wildcard { Vec::new() } else { missing };

    ExhaustivenessReport {
        is_exhaustive,
        missing_cases: report_missing,
        redundant_arms: Vec::new(),
    }
}

/// Extract a stable string key for a `Type::Union` variant.
///
/// Falls back to the `Debug` rendering for variant shapes we don't
/// explicitly recognise (e.g. inline tuples); those cases never appear in
/// the test suite but keep the function total.
fn variant_name(ty: &Type) -> String {
    match ty {
        Type::Named(name, _) => name.clone(),
        Type::Generic { base, .. } => base.clone(),
        Type::Byte => "byte".to_string(),
        other => format!("{:?}", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn union(names: &[&str]) -> Type {
        Type::Union(
            names
                .iter()
                .map(|n| Type::Named((*n).to_string(), Vec::new()))
                .collect(),
        )
    }

    #[test]
    fn missing_three_variant_with_one_arm() {
        let mut bag = DiagnosticBag::new();
        let ty = union(&["Red", "Green", "Blue"]);
        let report = check_union(&mut bag, &ty, &["Red".to_string()]);
        assert!(!report.is_exhaustive);
        assert_eq!(
            report.missing_cases,
            vec!["Green".to_string(), "Blue".to_string()]
        );
        assert!(bag.has_warnings());
    }

    #[test]
    fn wildcard_marks_exhaustive() {
        let mut bag = DiagnosticBag::new();
        let ty = union(&["Red", "Green", "Blue"]);
        let report = check_union(&mut bag, &ty, &["Red".to_string(), "_".to_string()]);
        assert!(report.is_exhaustive);
        assert!(report.missing_cases.is_empty());
        assert!(!bag.has_warnings());
    }

    #[test]
    fn full_coverage_no_warning() {
        let mut bag = DiagnosticBag::new();
        let ty = union(&["A", "B"]);
        let report = check_union(&mut bag, &ty, &["A".to_string(), "B".to_string()]);
        assert!(report.is_exhaustive);
        assert!(!bag.has_warnings());
    }

    #[test]
    fn non_union_subject_passes_through() {
        let mut bag = DiagnosticBag::new();
        let report = check_union(&mut bag, &Type::Int, &[]);
        assert!(report.is_exhaustive);
        assert!(report.missing_cases.is_empty());
        assert!(!bag.has_warnings());
    }
}
