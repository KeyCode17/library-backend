//! One flat error type the mobile side sees as a single exception.

use std::fmt;

#[derive(Debug, uniffi::Error)]
pub enum FfiError {
    /// An input could not be parsed (e.g. a malformed candidate id).
    Validation { msg: String },
}

impl fmt::Display for FfiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FfiError::Validation { msg } => write!(f, "validation error: {msg}"),
        }
    }
}

impl std::error::Error for FfiError {}
