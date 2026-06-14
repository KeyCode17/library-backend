use std::sync::Arc;

use uuid::Uuid;

use iam::domain::{AuthPrincipal, Permission};

use crate::domain::{BookGateway, Clock, LendingError, Loan, LoanRepository};

/// Use case: return a borrowed book. The owner may return their own loan; staff
/// (with `ManageLoans`) may return any. Flips the book available again.
pub struct ReturnLoan {
    loans: Arc<dyn LoanRepository>,
    books: Arc<dyn BookGateway>,
    clock: Arc<dyn Clock>,
}

impl ReturnLoan {
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
        loan_id: Uuid,
    ) -> Result<Loan, LendingError> {
        let mut loan = self
            .loans
            .find_by_id(loan_id)
            .await?
            .ok_or(LendingError::LoanNotFound)?;

        // Ownership check (no IDOR): only the borrower or staff may return.
        let is_owner = loan.user_id == actor.user_id;
        let is_staff = actor.role.grants(Permission::ManageLoans);
        if !is_owner && !is_staff {
            return Err(LendingError::Forbidden);
        }

        loan.mark_returned(self.clock.now())?;
        self.loans.update(loan.clone()).await?;
        self.books.set_available(loan.book_id, true).await?;
        Ok(loan)
    }
}
