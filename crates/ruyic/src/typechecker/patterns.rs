/**
 * Pattern analysis for exhaustiveness and redundancy checking.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use crate::parser::ast::Pattern;
use crate::typechecker::types::Type;
use std::collections::HashSet;

/// Result of pattern analysis.
#[derive(Debug, Clone)]
pub struct PatternAnalysis {
    pub is_exhaustive: bool,
    pub missing_cases: Vec<String>,
    pub has_redundancy: bool,
    pub redundant_arm: Option<usize>,
}

/// Analyzes a list of match arms for exhaustiveness and redundancy.
pub fn analyze_patterns(arms: &[(&Pattern, &Type)]) -> PatternAnalysis {
    if arms.is_empty() {
        return PatternAnalysis {
            is_exhaustive: false,
            missing_cases: vec!["any".to_string()],
            has_redundancy: false,
            redundant_arm: None,
        };
    }

    let scrutinee_type = arms[0].1;
    let mut all_covered: HashSet<String> = HashSet::new();
    let mut has_wildcard = false;
    let mut redundant_arm = None;

    for (i, (pattern, _)) in arms.iter().enumerate() {
        let covered = pattern_covered_cases(pattern, scrutinee_type);

        if covered.contains(&"_".to_string()) {
            has_wildcard = true;
        }

        for case in &covered {
            if all_covered.contains(case) && redundant_arm.is_none() {
                redundant_arm = Some(i);
            }
            all_covered.insert(case.clone());
        }
    }

    let missing_cases = find_missing_cases(scrutinee_type, &all_covered);

    PatternAnalysis {
        is_exhaustive: has_wildcard || missing_cases.is_empty(),
        missing_cases,
        has_redundancy: redundant_arm.is_some(),
        redundant_arm,
    }
}

/// Returns the set of cases that a pattern covers.
fn pattern_covered_cases(pattern: &Pattern, scrutinee_type: &Type) -> HashSet<String> {
    let mut cases = HashSet::new();

    match pattern {
        Pattern::Wildcard => {
            cases.insert("_".to_string());
        }
        Pattern::Identifier(name) => {
            cases.insert(name.clone());
            // Identifier patterns are exhaustive (bind any value)
            cases.insert("_".to_string());
        }
        Pattern::Literal(expr) => {
            let case = match expr.as_ref() {
                crate::parser::ast::Expr::BooleanLiteral(true) => "true".to_string(),
                crate::parser::ast::Expr::BooleanLiteral(false) => "false".to_string(),
                crate::parser::ast::Expr::NullLiteral => "null".to_string(),
                crate::parser::ast::Expr::IntLiteral(n) => format!("int:{}", n),
                crate::parser::ast::Expr::StringLiteral(s) => format!("string:{}", s),
                crate::parser::ast::Expr::FloatLiteral(f) => format!("float:{}", f),
                _ => format!("{:?}", expr),
            };
            cases.insert(case);
        }
        Pattern::Or(patterns) => {
            for p in patterns {
                cases.extend(pattern_covered_cases(p, scrutinee_type));
            }
        }
        Pattern::As(inner, alias) => {
            let inner_cases = pattern_covered_cases(inner, scrutinee_type);
            for case in inner_cases {
                cases.insert(format!("{} as {}", case, alias));
            }
        }
        Pattern::Object(fields) => {
            let field_list: Vec<String> = fields
                .iter()
                .map(|f| match f {
                    crate::parser::ast::ObjectPatternField::Property { key, .. } => key.clone(),
                    crate::parser::ast::ObjectPatternField::Shorthand(name) => name.clone(),
                    crate::parser::ast::ObjectPatternField::Rest(name) => format!("...{}", name),
                })
                .collect();
            cases.insert(format!("{{{}}}", field_list.join(", ")));
        }
        Pattern::Array(elements) => {
            let elem_list: Vec<String> = elements
                .iter()
                .map(|e| match e {
                    crate::parser::ast::ArrayPatternElement::Pattern(p) => {
                        let inner = pattern_covered_cases(p, scrutinee_type);
                        if inner.len() == 1 {
                            inner.iter().next().unwrap().clone()
                        } else {
                            "[pattern]".to_string()
                        }
                    }
                    crate::parser::ast::ArrayPatternElement::Rest(_) => "...".to_string(),
                    crate::parser::ast::ArrayPatternElement::Elision => "_".to_string(),
                })
                .collect();
            cases.insert(format!("[{}]", elem_list.join(", ")));
        }
        Pattern::Rest(name) => {
            cases.insert(format!("...{}", name));
        }
    }

    cases
}

/// Finds missing cases for exhaustiveness checking.
fn find_missing_cases(scrutinee_type: &Type, covered: &HashSet<String>) -> Vec<String> {
    match scrutinee_type {
        Type::Int | Type::BigInt => {
            if covered.contains("_") || covered.is_empty() {
                vec![]
            } else {
                vec!["<other int value>".to_string()]
            }
        }
        Type::Float => {
            if covered.contains("_") || covered.is_empty() {
                vec![]
            } else {
                vec!["<other float value>".to_string()]
            }
        }
        Type::String => {
            if covered.contains("_") || covered.is_empty() {
                vec![]
            } else {
                vec!["<other string value>".to_string()]
            }
        }
        Type::Bool => {
            let mut missing = vec![];
            if !covered.contains("true") && !covered.contains("_") {
                missing.push("true".to_string());
            }
            if !covered.contains("false") && !covered.contains("_") {
                missing.push("false".to_string());
            }
            missing
        }
        Type::Null => {
            if !covered.contains("null") && !covered.contains("_") {
                vec!["null".to_string()]
            } else {
                vec![]
            }
        }
        Type::Array(_) => {
            if covered.contains("_") || covered.contains("[]") {
                vec![]
            } else {
                vec!["[...]".to_string()]
            }
        }
        Type::Object(fields) => {
            let mut missing = vec![];
            for field in fields {
                let field_name = &field.name;
                if !covered
                    .iter()
                    .any(|c| c.contains(&format!("{}:", field_name)))
                {
                    missing.push(format!(".{}", field_name));
                }
            }
            missing
        }
        Type::Nullable(inner) => {
            let inner_missing = find_missing_cases(inner, covered);
            let mut all_missing = inner_missing;
            if !covered.contains("null") && !covered.contains("_") {
                all_missing.push("null".to_string());
            }
            all_missing
        }
        Type::Named(name) => {
            if covered.contains("_") || covered.contains(name) {
                vec![]
            } else {
                vec![name.clone()]
            }
        }
        _ => {
            if covered.contains("_") {
                vec![]
            } else {
                vec!["<other>".to_string()]
            }
        }
    }
}

/// Checks if a pattern is refutable (can fail to match).
pub fn is_refutable(pattern: &Pattern) -> bool {
    !matches!(pattern, Pattern::Wildcard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{ArrayPatternElement, ObjectPatternField};
    use crate::typechecker::types::ObjectField;

    #[test]
    fn test_wildcard_is_exhaustive() {
        let pattern = Pattern::Wildcard;
        let ty = Type::Int;
        let cases = pattern_covered_cases(&pattern, &ty);
        assert!(cases.contains("_"));
    }

    #[test]
    fn test_bool_patterns_both_covered() {
        let pattern_true =
            Pattern::Literal(Box::new(crate::parser::ast::Expr::BooleanLiteral(true)));
        let pattern_false =
            Pattern::Literal(Box::new(crate::parser::ast::Expr::BooleanLiteral(false)));
        let arms = vec![(&pattern_true, &Type::Bool), (&pattern_false, &Type::Bool)];
        let result = analyze_patterns(&arms);
        assert!(result.is_exhaustive);
        assert!(!result.has_redundancy);
    }

    #[test]
    fn test_bool_patterns_with_wildcard() {
        let pattern_true =
            Pattern::Literal(Box::new(crate::parser::ast::Expr::BooleanLiteral(true)));
        let arms = vec![
            (&pattern_true, &Type::Bool),
            (&Pattern::Wildcard, &Type::Bool),
        ];
        let result = analyze_patterns(&arms);
        assert!(result.is_exhaustive);
        assert!(result.has_redundancy);
        assert_eq!(result.redundant_arm, Some(1));
    }

    #[test]
    fn test_null_pattern() {
        let pattern_null = Pattern::Literal(Box::new(crate::parser::ast::Expr::NullLiteral));
        let arms = vec![(&pattern_null, &Type::Null)];
        let result = analyze_patterns(&arms);
        assert!(result.is_exhaustive);
    }

    #[test]
    fn test_object_pattern_exhaustiveness() {
        let obj_type = Type::Object(vec![
            ObjectField {
                name: "x".to_string(),
                ty: Type::Int,
                optional: false,
            },
            ObjectField {
                name: "y".to_string(),
                ty: Type::Int,
                optional: false,
            },
        ]);
        let pattern = Pattern::Object(vec![
            ObjectPatternField::Property {
                key: "x".to_string(),
                pattern: Pattern::Wildcard,
            },
            ObjectPatternField::Property {
                key: "y".to_string(),
                pattern: Pattern::Wildcard,
            },
        ]);
        let cases = pattern_covered_cases(&pattern, &obj_type);
        assert!(!cases.is_empty());
    }

    #[test]
    fn test_array_pattern_exhaustiveness() {
        let arr_type = Type::Array(Box::new(Type::Int));
        let pattern = Pattern::Array(vec![
            ArrayPatternElement::Pattern(Pattern::Wildcard),
            ArrayPatternElement::Rest(Pattern::Identifier("rest".to_string())),
        ]);
        let cases = pattern_covered_cases(&pattern, &arr_type);
        assert!(!cases.is_empty());
    }

    #[test]
    fn test_or_pattern() {
        let pattern1 = Pattern::Literal(Box::new(crate::parser::ast::Expr::IntLiteral(1)));
        let pattern2 = Pattern::Literal(Box::new(crate::parser::ast::Expr::IntLiteral(2)));
        let pattern = Pattern::Or(vec![pattern1, pattern2]);
        let ty = Type::Int;
        let cases = pattern_covered_cases(&pattern, &ty);
        assert_eq!(cases.len(), 2);
    }

    #[test]
    fn test_identifier_pattern() {
        let pattern = Pattern::Identifier("x".to_string());
        let ty = Type::Int;
        let cases = pattern_covered_cases(&pattern, &ty);
        assert!(cases.contains(&"x".to_string()));
    }
}
