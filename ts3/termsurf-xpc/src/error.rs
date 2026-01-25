//! Error types for XPC operations.

use std::fmt;

/// Errors that can occur during XPC operations.
#[derive(Debug, Clone)]
pub enum XpcError {
    /// The connection was interrupted (remote end crashed or was killed).
    ConnectionInterrupted,

    /// The connection is invalid (service not found or connection cancelled).
    ConnectionInvalid,

    /// The service is about to be terminated.
    TerminationImminent,

    /// A null pointer was received where a valid object was expected.
    NullPointer(&'static str),

    /// Type mismatch (expected one XPC type, got another).
    TypeMismatch {
        expected: &'static str,
        context: &'static str,
    },

    /// An unknown error occurred.
    Unknown(String),
}

impl fmt::Display for XpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XpcError::ConnectionInterrupted => {
                write!(f, "XPC connection interrupted")
            }
            XpcError::ConnectionInvalid => {
                write!(f, "XPC connection invalid")
            }
            XpcError::TerminationImminent => {
                write!(f, "XPC service termination imminent")
            }
            XpcError::NullPointer(context) => {
                write!(f, "XPC null pointer: {}", context)
            }
            XpcError::TypeMismatch { expected, context } => {
                write!(f, "XPC type mismatch: expected {} in {}", expected, context)
            }
            XpcError::Unknown(msg) => {
                write!(f, "XPC error: {}", msg)
            }
        }
    }
}

impl std::error::Error for XpcError {}

pub type Result<T> = std::result::Result<T, XpcError>;
