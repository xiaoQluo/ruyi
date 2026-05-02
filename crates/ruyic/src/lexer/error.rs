/**
 * Lexer error types with source location information.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum LexerError {
    #[error("Invalid character '{ch}' at line {line}, column {col}")]
    InvalidCharacter { ch: char, line: usize, col: usize },

    #[error("Unterminated string at line {line}, column {col}")]
    UnterminatedString { line: usize, col: usize },

    #[error("Unterminated comment at line {line}, column {col}")]
    UnterminatedComment { line: usize, col: usize },

    #[error("Unterminated template string at line {line}, column {col}")]
    UnterminatedTemplate { line: usize, col: usize },

    #[error("Invalid escape sequence at line {line}, column {col}")]
    InvalidEscape { line: usize, col: usize },

    #[error("Invalid numeric literal at line {line}, column {col}: {msg}")]
    InvalidNumber { line: usize, col: usize, msg: String },

    #[error("Unexpected end of file at line {line}, column {col}")]
    UnexpectedEof { line: usize, col: usize },
}
