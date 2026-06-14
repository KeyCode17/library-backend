//! Lending domain errors.

use std::fmt;

#[derive(Debug)]
pub enum LendingError {
    /// The referenced book does not exist.
    BookNotFound,
    /// The book is already on loan / not available.
    BookUnavailable,
    /// The referenced loan does not exist.
    LoanNotFound,
    /// The caller may not act on this loan (not the owner / lacks the role).
    Forbidden,
    /// The loan is not in a state that allows this transition.
    InvalidState,
    /// A dependency (e.g. the catalog) failed.
    Dependency(String),
    /// The loan store failed.
    Repository(String),
}

impl fmt::Display for LendingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            LendingError::BookNotFound => "book not found",
            LendingError::BookUnavailable => "book unavailable",
            LendingError::LoanNotFound => "loan not found",
            LendingError::Forbidden => "forbidden",
            LendingError::InvalidState => "invalid loan state for this action",
            LendingError::Dependency(_) => "dependency failure",
            LendingError::Repository(_) => "repository failure",
        };
        f.write_str(message)
    }
}

impl std::error::Error for LendingError {}
