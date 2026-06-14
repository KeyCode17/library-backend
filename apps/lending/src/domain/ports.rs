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
    /// All currently-borrowed (active) loans, for due-date scanning. Consumed by
    /// the notification scheduler through a gateway-owned bridge.
    async fn list_active(&self) -> Result<Vec<Loan>, LendingError>;
}

/// Outcome of an atomic borrow-claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimOutcome {
    /// The book was available and is now claimed for this borrower.
    Claimed,
    /// The book exists but was already unavailable.
    Unavailable,
    /// No book with this id.
    NotFound,
}

/// Port to the catalog's book availability. Abstract so the lending domain does
/// not depend on `catalog`; the gateway provides the bridging adapter.
#[async_trait]
pub trait BookGateway: Send + Sync {
    /// Atomically claim a book for borrowing: it transitions to unavailable iff it
    /// was available. The single atomic claim (not a separate check-then-set) is
    /// what makes borrow race-free.
    async fn claim_for_borrow(&self, book_id: Uuid) -> Result<ClaimOutcome, LendingError>;
    /// Release a book's availability (on return, or to undo a failed borrow).
    async fn set_available(&self, book_id: Uuid, available: bool) -> Result<(), LendingError>;
}

/// Clock port, so use cases are not bound to the wall clock.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}
