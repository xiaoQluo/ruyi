use thiserror::Error;

#[derive(Error, Debug)]
pub enum TypeError {
    #[error("Type mismatch: expected {expected} but found {found}")]
    TypeMismatch { expected: String, found: String },

    #[error("Unknown variable {name}")]
    UnknownVariable { name: String },

    #[error("Cannot assign to immutable variable {name}")]
    ImmutableAssign { name: String },

    #[error("Nullable access on non-nullable type")]
    NullableError,
}