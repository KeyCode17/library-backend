//! Ports the lending use cases depend on.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::error::LendingError;
use super::loan::Loan;
use super::pagination::{Page, PageRequest};

/// Persistence port for loans.
#[async_trait]
pub trait LoanRepository: Send + Sync {
    async fn insert(&self, loan: Loan) -> Result<(), LendingError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Loan>, LendingError>;
    /// Replace an existing loan (matched by id). Errors `LoanNotFound` if absent.
    async fn update(&self, loan: Loan) -> Result<(), LendingError>;
    async fn list_all(&self, request: PageRequest) -> Result<Page<Loan>, LendingError>;
    async fn list_for_user(
        &self,
        user_id: Uuid,
        request: PageRequest,
    ) -> Result<Page<Loan>, LendingError>;
}

/// Port to the catalog's book availability. Abstract so the lending domain does
/// not depend on `catalog`; the gateway provides the bridging adapter.
#[async_trait]
pub trait BookGateway: Send + Sync {
    /// `Some(available)` for an existing book, `None` if the book does not exist.
    async fn is_available(&self, book_id: Uuid) -> Result<Option<bool>, LendingError>;
    /// Flip a book's availability.
    async fn set_available(&self, book_id: Uuid, available: bool) -> Result<(), LendingError>;
}

/// Clock port, so use cases are not bound to the wall clock.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}
