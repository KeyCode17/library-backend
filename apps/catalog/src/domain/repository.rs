use async_trait::async_trait;
use uuid::Uuid;

use super::book::Book;
use super::error::RepositoryError;
use super::filter::BookFilter;
use super::pagination::{Page, PageRequest};

/// Outcome of an atomic borrow-claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimOutcome {
    /// The book was available and is now marked unavailable for this borrower.
    Claimed,
    /// The book exists but was already unavailable.
    Unavailable,
    /// No book with this id.
    NotFound,
}

/// Port for books. Implemented by infrastructure adapters (in-memory and
/// Postgres/SeaORM). Async because the real adapter is I/O bound (ADR 0003).
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
    async fn set_availability(
        &self,
        id: Uuid,
        available: bool,
    ) -> Result<Option<Book>, RepositoryError>;

    /// Atomically claim a book for borrowing: flip it to unavailable **iff** it is
    /// currently available, reporting what happened. This single conditional
    /// update is the atomicity primitive that fixes the borrow TOCTOU — two
    /// concurrent claims cannot both succeed.
    async fn claim_if_available(&self, id: Uuid) -> Result<ClaimOutcome, RepositoryError>;
}
