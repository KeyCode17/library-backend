//! Chat domain errors.

use std::fmt;

#[derive(Debug)]
pub enum ChatError {
    /// A message with no (non-whitespace) body.
    EmptyBody,
    /// The message store failed.
    Repository(String),
}

impl fmt::Display for ChatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            ChatError::EmptyBody => "message body is empty",
            ChatError::Repository(_) => "repository failure",
        };
        f.write_str(message)
    }
}

impl std::error::Error for ChatError {}
