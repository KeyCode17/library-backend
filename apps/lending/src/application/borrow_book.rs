use std::sync::Arc;

use chrono::Duration;
use uuid::Uuid;

use iam::domain::{AuthPrincipal, Permission};

use crate::domain::{BookGateway, ClaimOutcome, Clock, LendingError, Loan, LoanRepository};

/// Standard loan period.
pub const LOAN_PERIOD_DAYS: i64 = 14;

/// Use case: borrow a book. Opens an active loan and flips the book unavailable.
pub struct BorrowBook {
    loans: Arc<dyn LoanRepository>,
    books: Arc<dyn BookGateway>,
    clock: Arc<dyn Clock>,
}

impl BorrowBook {
    pub fn new(
        loans: Arc<dyn LoanRepository>,
        books: Arc<dyn BookGateway>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            loans,
            books,
            clock,
        }
    }

    pub async fn execute(
        &self,
        actor: &AuthPrincipal,
        book_id: Uuid,
    ) -> Result<Loan, LendingError> {
        if !actor.role.grants(Permission::BorrowBooks) {
            return Err(LendingError::Forbidden);
        }

        // Atomic claim: only one concurrent borrow of a given book can win here.
        match self.books.claim_for_borrow(book_id).await? {
            ClaimOutcome::NotFound => return Err(LendingError::BookNotFound),
            ClaimOutcome::Unavailable => return Err(LendingError::BookUnavailable),
            ClaimOutcome::Claimed => {}
        }

        let now = self.clock.now();
        let due = now + Duration::days(LOAN_PERIOD_DAYS);
        let loan = Loan::borrow(Uuid::new_v4(), book_id, actor.user_id, now, due);

        if let Err(error) = self.loans.insert(loan.clone()).await {
            // The book was claimed but the loan didn't persist — release it.
            let _ = self.books.set_available(book_id, true).await;
            return Err(error);
        }
        Ok(loan)
    }
}
