/**
 * Diagnostic types for type checker error reporting.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use crate::typechecker::types::Type;
use std::fmt;

/// Severity level for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

/// A type checker diagnostic message.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    message: String,
    ty: DiagnosticKind,
}

/// Categorized diagnostic kinds for type errors.
#[derive(Debug, Clone)]
pub enum DiagnosticKind {
    TypeMismatch {
        expected: Type,
        found: Type,
    },
    UnknownVariable {
        name: String,
    },
    ImmutableAssign {
        name: String,
    },
    NullableAccess {
        ty: Type,
    },
    UnsafeNullableAccess {
        ty: Type,
    },
    NotCallable {
        ty: Type,
    },
    NotIndexable {
        ty: Type,
    },
    ArgumentCount {
        expected: usize,
        found: usize,
    },
    MissingReturn {
        function: String,
    },
    UnreachableCode,
    DynCast {
        from: Type,
        to: Type,
    },
    RecursiveTypeAlias {
        name: String,
    },
    DuplicateDeclaration {
        name: String,
    },
    TraitNotImplemented {
        ty: Type,
        trait_name: String,
    },
    GenericArity {
        name: String,
        expected: usize,
        found: usize,
    },
    CannotInfer,
    // Pattern matching diagnostics
    NonExhaustiveMatch {
        scrutinee_type: Type,
        missing: Vec<String>,
    },
    RedundantPattern {
        arm: usize,
    },
    PatternTypeMismatch {
        pattern: String,
        expected: Type,
        found: Type,
    },
    InvalidPattern {
        message: String,
    },
    Other {
        message: String,
    },
}

impl Diagnostic {
    pub fn error(kind: DiagnosticKind) -> Self {
        let message = kind.message();
        Self {
            severity: Severity::Error,
            message,
            ty: kind,
        }
    }

    pub fn warning(kind: DiagnosticKind) -> Self {
        let message = kind.message();
        Self {
            severity: Severity::Warning,
            message,
            ty: kind,
        }
    }

    pub fn note(msg: &str) -> Self {
        Self {
            severity: Severity::Note,
            message: msg.to_string(),
            ty: DiagnosticKind::Other {
                message: msg.to_string(),
            },
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self.severity, Severity::Error)
    }

    pub fn is_warning(&self) -> bool {
        matches!(self.severity, Severity::Warning)
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn kind(&self) -> &DiagnosticKind {
        &self.ty
    }
}

impl DiagnosticKind {
    fn message(&self) -> String {
        match self {
            DiagnosticKind::TypeMismatch { expected, found } => {
                format!(
                    "Type mismatch: expected `{}`, but found `{}`",
                    expected, found
                )
            }
            DiagnosticKind::UnknownVariable { name } => {
                format!("Unknown variable: `{}`", name)
            }
            DiagnosticKind::ImmutableAssign { name } => {
                format!("Cannot assign to immutable variable `{}`", name)
            }
            DiagnosticKind::NullableAccess { ty } => {
                format!("Nullable access on type `{}` without null check", ty)
            }
            DiagnosticKind::UnsafeNullableAccess { ty } => {
                format!("Unsafe nullable access: member access on `{}` requires null check or optional chaining (`?.`)", ty)
            }
            DiagnosticKind::NotCallable { ty } => {
                format!("Type `{}` is not callable", ty)
            }
            DiagnosticKind::NotIndexable { ty } => {
                format!("Type `{}` is not indexable", ty)
            }
            DiagnosticKind::ArgumentCount { expected, found } => {
                format!("Expected {} argument(s), but found {}", expected, found)
            }
            DiagnosticKind::MissingReturn { function } => {
                format!("Function `{}` may not return a value", function)
            }
            DiagnosticKind::UnreachableCode => "Unreachable code detected".to_string(),
            DiagnosticKind::DynCast { from, to } => {
                format!("Runtime cast from `{}` to `{}` will be inserted", from, to)
            }
            DiagnosticKind::RecursiveTypeAlias { name } => {
                format!("Recursive type alias: `{}`", name)
            }
            DiagnosticKind::DuplicateDeclaration { name } => {
                format!("Duplicate declaration: `{}`", name)
            }
            DiagnosticKind::TraitNotImplemented { ty, trait_name } => {
                format!("Type `{}` does not implement trait `{}`", ty, trait_name)
            }
            DiagnosticKind::GenericArity {
                name,
                expected,
                found,
            } => {
                format!(
                    "Generic `{}` expects {} type argument(s), but found {}",
                    name, expected, found
                )
            }
            DiagnosticKind::CannotInfer => "Cannot infer type; defaulting to `dyn`".to_string(),
            DiagnosticKind::NonExhaustiveMatch {
                scrutinee_type,
                missing,
            } => {
                format!(
                    "Non-exhaustive match: type `{}` has unscovered cases: {}",
                    scrutinee_type,
                    missing.join(", ")
                )
            }
            DiagnosticKind::RedundantPattern { arm } => {
                format!("Redundant pattern at arm {}", arm)
            }
            DiagnosticKind::PatternTypeMismatch {
                pattern,
                expected,
                found,
            } => {
                format!(
                    "Pattern `{}` cannot match value of type `{}`: expected `{}`",
                    pattern, found, expected
                )
            }
            DiagnosticKind::InvalidPattern { message } => {
                format!("Invalid pattern: {}", message)
            }
            DiagnosticKind::Other { message } => message.clone(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        };
        write!(f, "{}: {}", prefix, self.message)
    }
}

/// Collection of diagnostics produced during type checking.
#[derive(Debug, Clone, Default)]
pub struct DiagnosticBag {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticBag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn add_error(&mut self, kind: DiagnosticKind) {
        self.add(Diagnostic::error(kind));
    }

    pub fn add_warning(&mut self, kind: DiagnosticKind) {
        self.add(Diagnostic::warning(kind));
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }

    pub fn has_warnings(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_warning())
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter().filter(|d| d.is_error())
    }

    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter().filter(|d| d.is_warning())
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_mismatch_diagnostic() {
        let diag = Diagnostic::error(DiagnosticKind::TypeMismatch {
            expected: Type::Int,
            found: Type::String,
        });
        assert!(diag.is_error());
        assert_eq!(
            diag.message(),
            "Type mismatch: expected `int`, but found `string`"
        );
    }

    #[test]
    fn test_unknown_variable_diagnostic() {
        let diag = Diagnostic::error(DiagnosticKind::UnknownVariable { name: "x".into() });
        assert_eq!(diag.message(), "Unknown variable: `x`");
    }

    #[test]
    fn test_diagnostic_bag() {
        let mut bag = DiagnosticBag::new();
        bag.add_error(DiagnosticKind::TypeMismatch {
            expected: Type::Int,
            found: Type::String,
        });
        bag.add_warning(DiagnosticKind::CannotInfer);
        assert!(bag.has_errors());
        assert!(bag.has_warnings());
        assert_eq!(bag.diagnostics().len(), 2);
    }

    #[test]
    fn test_diagnostic_display() {
        let diag = Diagnostic::error(DiagnosticKind::ImmutableAssign { name: "PI".into() });
        let s = format!("{}", diag);
        assert!(s.starts_with("error:"));
    }
}
