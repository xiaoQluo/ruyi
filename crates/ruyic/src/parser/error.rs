/**
 * Parser error types with source location information.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum ParseError {
    #[error("Unexpected token '{token}' at line {line}, column {col}")]
    UnexpectedToken {
        token: String,
        line: usize,
        col: usize,
    },

    #[error("Expected {expected} but found '{found}' at line {line}, column {col}")]
    ExpectedToken {
        expected: String,
        found: String,
        line: usize,
        col: usize,
    },

    #[error("{message} at line {line}, column {col}")]
    SyntaxError {
        message: String,
        line: usize,
        col: usize,
    },

    #[error("Unexpected end of file at line {line}, column {col}")]
    UnexpectedEof { line: usize, col: usize },

    #[error("Lexer error: {0}")]
    LexerError(String),
}
