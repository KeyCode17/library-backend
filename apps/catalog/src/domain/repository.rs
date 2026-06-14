use async_trait::async_trait;

use super::book::Book;
use super::error::RepositoryError;
use super::pagination::{Page, PageRequest};

/// Port for reading books. Implemented by infrastructure adapters (in-memory
/// today, Postgres/SeaORM once the DB is wired). Async because the real adapter
/// is I/O bound (ADR 0003: Axum + Postgres).
#[async_trait]
pub trait BookRepository: Send + Sync {
    /// Return a page of books ordered deterministically, plus the total count.
    async fn list(&self, request: PageRequest) -> Result<Page<Book>, RepositoryError>;
}
