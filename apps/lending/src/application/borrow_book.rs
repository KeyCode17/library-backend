use std::sync::Arc;

use chrono::Duration;
use uuid::Uuid;

use iam::domain::{AuthPrincipal, Permission};

use crate::domain::{BookGateway, Clock, LendingError, Loan, LoanRepository};

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

        match self.books.is_available(book_id).await? {
            None => return Err(LendingError::BookNotFound),
            Some(false) => return Err(LendingError::BookUnavailable),
            Some(true) => {}
        }

        let now = self.clock.now();
        let due = now + Duration::days(LOAN_PERIOD_DAYS);
        let loan = Loan::borrow(Uuid::new_v4(), book_id, actor.user_id, now, due);

        self.loans.insert(loan.clone()).await?;
        self.books.set_available(book_id, false).await?;
        Ok(loan)
    }
}
