//! Notification domain errors.

use std::fmt;

#[derive(Debug)]
pub enum NotificationError {
    /// A device token was empty/invalid.
    InvalidToken,
    /// A store failed.
    Repository(String),
    /// A dependency (e.g. the loan source) failed.
    Dependency(String),
    /// The push transport failed.
    Push(String),
}

impl fmt::Display for NotificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            NotificationError::InvalidToken => "invalid device token",
            NotificationError::Repository(_) => "repository failure",
            NotificationError::Dependency(_) => "dependency failure",
            NotificationError::Push(_) => "push failure",
        };
        f.write_str(message)
    }
}

impl std::error::Error for NotificationError {}
