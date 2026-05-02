/**
 * Error code system for Ruyi compiler diagnostics.
 *
 * Error codes follow the format: Category + Number
 * Categories:
 *   E1xxx - Lexical errors
 *   E2xxx - Syntax errors
 *   E3xxx - Type errors
 *   E4xxx - Resolution errors
 *   W1xxx - Warnings
 *
 * @author Ruyi Team
 * @date 2026-05-02
 */
use std::fmt;

/// Error code categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// Lexical analysis errors (E1xxx)
    Lexical,
    /// Syntax parsing errors (E2xxx)
    Syntax,
    /// Type checking errors (E3xxx)
    Type,
    /// Name/visibility resolution errors (E4xxx)
    Resolution,
    /// Warnings (W1xxx)
    Warning,
}

impl ErrorCategory {
    /// Get the prefix for this category.
    pub fn prefix(&self) -> &'static str {
        match self {
            ErrorCategory::Lexical => "E1",
            ErrorCategory::Syntax => "E2",
            ErrorCategory::Type => "E3",
            ErrorCategory::Resolution => "E4",
            ErrorCategory::Warning => "W1",
        }
    }
}

/// A structured error code with category and number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorCode {
    pub category: ErrorCategory,
    pub number: u16,
}

impl ErrorCode {
    /// Create a new error code.
    pub fn new(category: ErrorCategory, number: u16) -> Self {
        Self { category, number }
    }

    /// Get the string representation like "E0001", "W0001".
    pub fn as_str(&self) -> String {
        format!("{}{:04}", self.category.prefix(), self.number)
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =============================================================================
// LEXICAL ERROR CODES (E1xxx)
// =============================================================================

/// Lexical error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum LexicalErrorCode {
    /// E1001: Invalid character encountered
    InvalidCharacter = 1001,
    /// E1002: Unterminated string literal
    UnterminatedString = 1002,
    /// E1003: Unterminated comment
    UnterminatedComment = 1003,
    /// E1004: Unterminated template string
    UnterminatedTemplate = 1004,
    /// E1005: Invalid escape sequence
    InvalidEscape = 1005,
    /// E1006: Invalid numeric literal
    InvalidNumber = 1006,
    /// E1007: Unexpected end of file
    UnexpectedEof = 1007,
}

impl LexicalErrorCode {
    /// Convert to general ErrorCode.
    pub fn to_error_code(&self) -> ErrorCode {
        ErrorCode::new(ErrorCategory::Lexical, *self as u16)
    }
}

// =============================================================================
// SYNTAX ERROR CODES (E2xxx)
// =============================================================================

/// Syntax error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum SyntaxErrorCode {
    /// E2001: Unexpected token
    UnexpectedToken = 2001,
    /// E2002: Expected token not found
    ExpectedToken = 2002,
    /// E2003: Syntax error
    SyntaxError = 2003,
    /// E2004: Unexpected end of file
    UnexpectedEof = 2004,
    /// E2005: Unmatched closing delimiter
    UnmatchedDelimiter = 2005,
    /// E2006: Missing semicolon
    MissingSemicolon = 2006,
    /// E2007: Invalid expression statement
    InvalidExpression = 2007,
}

impl SyntaxErrorCode {
    /// Convert to general ErrorCode.
    pub fn to_error_code(&self) -> ErrorCode {
        ErrorCode::new(ErrorCategory::Syntax, *self as u16)
    }
}

// =============================================================================
// TYPE ERROR CODES (E3xxx)
// =============================================================================

/// Type error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum TypeErrorCode {
    /// E3001: Type mismatch
    TypeMismatch = 3001,
    /// E3002: Unknown type
    UnknownType = 3002,
    /// E3003: Not callable
    NotCallable = 3003,
    /// E3004: Not indexable
    NotIndexable = 3004,
    /// E3005: Argument count mismatch
    ArgumentCount = 3005,
    /// E3006: Missing return value
    MissingReturn = 3006,
    /// E3007: Cannot infer type
    CannotInfer = 3007,
    /// E3008: Nullable access without check
    UnsafeNullableAccess = 3008,
    /// E3009: Immutable assignment
    ImmutableAssign = 3009,
    /// E3010: Trait not implemented
    TraitNotImplemented = 3010,
    /// E3011: Generic arity mismatch
    GenericArity = 3011,
    /// E3012: Recursive type alias
    RecursiveTypeAlias = 3012,
}

impl TypeErrorCode {
    /// Convert to general ErrorCode.
    pub fn to_error_code(&self) -> ErrorCode {
        ErrorCode::new(ErrorCategory::Type, *self as u16)
    }
}

// =============================================================================
// RESOLUTION ERROR CODES (E4xxx)
// =============================================================================

/// Resolution error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ResolutionErrorCode {
    /// E4001: Unknown variable
    UnknownVariable = 4001,
    /// E4002: Unknown function
    UnknownFunction = 4002,
    /// E4003: Unknown field
    UnknownField = 4003,
    /// E4004: Duplicate declaration
    DuplicateDeclaration = 4004,
    /// E4005: Invalid import
    InvalidImport = 4005,
    /// E4006: Unresolved reference
    UnresolvedRef = 4006,
    /// E4007: Invalid module path
    InvalidModulePath = 4007,
}

impl ResolutionErrorCode {
    /// Convert to general ErrorCode.
    pub fn to_error_code(&self) -> ErrorCode {
        ErrorCode::new(ErrorCategory::Resolution, *self as u16)
    }
}

// =============================================================================
// WARNING CODES (W1xxx)
// =============================================================================

/// Warning codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum WarningCode {
    /// W1001: Unused variable
    UnusedVariable = 1001,
    /// W1002: Unreachable code
    UnreachableCode = 1002,
    /// W1003: Unused import
    UnusedImport = 1003,
    /// W1004: Unnecessary type cast
    UnnecessaryCast = 1004,
    /// W1005: Dead code
    DeadCode = 1005,
    /// W1006: Unused function parameter
    UnusedParameter = 1006,
    /// W1007: Implicit copy
    ImplicitCopy = 1007,
}

impl WarningCode {
    /// Convert to general ErrorCode.
    pub fn to_error_code(&self) -> ErrorCode {
        ErrorCode::new(ErrorCategory::Warning, *self as u16)
    }
}

// =============================================================================
// ERROR INDEX - Documentation reference
// =============================================================================

/// Error code index for documentation purposes.
/// Each entry contains the code, short name, and description.
pub static ERROR_INDEX: &[(&str, &str, &str)] = &[
    // Lexical errors
    (
        "E1001",
        "invalid-character",
        "Invalid character encountered",
    ),
    (
        "E1002",
        "unterminated-string",
        "Unterminated string literal",
    ),
    ("E1003", "unterminated-comment", "Unterminated comment"),
    (
        "E1004",
        "unterminated-template",
        "Unterminated template string",
    ),
    ("E1005", "invalid-escape", "Invalid escape sequence"),
    ("E1006", "invalid-number", "Invalid numeric literal"),
    ("E1007", "unexpected-eof", "Unexpected end of file"),
    // Syntax errors
    ("E2001", "unexpected-token", "Unexpected token"),
    ("E2002", "expected-token", "Expected token not found"),
    ("E2003", "syntax-error", "Syntax error"),
    ("E2004", "unexpected-eof", "Unexpected end of file"),
    (
        "E2005",
        "unmatched-delimiter",
        "Unmatched closing delimiter",
    ),
    ("E2006", "missing-semicolon", "Missing semicolon"),
    (
        "E2007",
        "invalid-expression",
        "Invalid expression statement",
    ),
    // Type errors
    ("E3001", "type-mismatch", "Type mismatch"),
    ("E3002", "unknown-type", "Unknown type"),
    ("E3003", "not-callable", "Value is not callable"),
    ("E3004", "not-indexable", "Value is not indexable"),
    ("E3005", "argument-count", "Argument count mismatch"),
    ("E3006", "missing-return", "Function may not return a value"),
    ("E3007", "cannot-infer", "Cannot infer type"),
    (
        "E3008",
        "unsafe-nullable-access",
        "Nullable access without null check",
    ),
    (
        "E3009",
        "immutable-assign",
        "Cannot assign to immutable variable",
    ),
    ("E3010", "trait-not-implemented", "Trait not implemented"),
    ("E3011", "generic-arity", "Generic parameter count mismatch"),
    ("E3012", "recursive-type-alias", "Recursive type alias"),
    // Resolution errors
    ("E4001", "unknown-variable", "Unknown variable"),
    ("E4002", "unknown-function", "Unknown function"),
    ("E4003", "unknown-field", "Unknown field"),
    ("E4004", "duplicate-declaration", "Duplicate declaration"),
    ("E4005", "invalid-import", "Invalid import"),
    ("E4006", "unresolved-ref", "Unresolved reference"),
    ("E4007", "invalid-module-path", "Invalid module path"),
    // Warnings
    ("W1001", "unused-variable", "Unused variable"),
    ("W1002", "unreachable-code", "Unreachable code"),
    ("W1003", "unused-import", "Unused import"),
    ("W1004", "unnecessary-cast", "Unnecessary type cast"),
    ("W1005", "dead-code", "Dead code"),
    ("W1006", "unused-parameter", "Unused function parameter"),
    ("W1007", "implicit-copy", "Implicit copy of non-Copy type"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        let code = ErrorCode::new(ErrorCategory::Type, 3001);
        assert_eq!(code.as_str(), "E3001");
        assert_eq!(format!("{}", code), "E3001");
    }

    #[test]
    fn test_lexical_error_code() {
        let code = LexicalErrorCode::UnterminatedString;
        assert_eq!(code.to_error_code().as_str(), "E1002");
    }

    #[test]
    fn test_warning_code() {
        let code = WarningCode::UnusedVariable;
        assert_eq!(code.to_error_code().as_str(), "W1001");
    }

    #[test]
    fn test_error_index_count() {
        // Verify all error codes are documented
        assert!(ERROR_INDEX.len() >= 25);
    }
}
