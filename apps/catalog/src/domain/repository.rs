use async_trait::async_trait;
use uuid::Uuid;

use super::book::Book;
use super::error::RepositoryError;
use super::filter::BookFilter;
use super::pagination::{Page, PageRequest};

/// Port for books. Implemented by infrastructure adapters (in-memory today,
/// Postgres/SeaORM once the DB is wired). Async because the real adapter is I/O
/// bound (ADR 0003: Axum + Postgres).
#[async_trait]
pub trait BookRepository: Send + Sync {
    /// Return a page of books matching `filter`, ordered deterministically, plus
    /// the total count of matches (the total is post-filter, pre-pagination).
    async fn list(
        &self,
        filter: &BookFilter,
        request: PageRequest,
    ) -> Result<Page<Book>, RepositoryError>;

    /// Return the book with `id`, or `None` if there is no such book.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Book>, RepositoryError>;

    /// Set a book's availability; returns the updated book, or `None` if absent.
    ///
    /// Used by the lending context (through a gateway-owned adapter) to flip a
    /// book unavailable on borrow and available again on return. Catalog itself
    /// stays unaware of lending.
    async fn set_availability(
        &self,
        id: Uuid,
        available: bool,
    ) -> Result<Option<Book>, RepositoryError>;
}
