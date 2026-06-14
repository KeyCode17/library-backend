use std::fmt;

/// Failure returned by a `BookRepository`. The presentation layer maps this to a
/// generic `500` with a flat `{ "error" }` body (FSD §4); the cause is logged,
/// never leaked to the client.
#[derive(Debug)]
pub enum RepositoryError {
    /// The backing store failed (connection, query, deserialization, ...).
    Backend(String),
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepositoryError::Backend(message) => {
                write!(f, "repository backend error: {message}")
            }
        }
    }
}

impl std::error::Error for RepositoryError {}
