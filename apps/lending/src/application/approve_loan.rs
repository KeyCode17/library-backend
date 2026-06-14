use std::sync::Arc;

use uuid::Uuid;

use iam::domain::{AuthPrincipal, Permission};

use crate::domain::{Clock, LendingError, Loan, LoanRepository};

/// Use case: staff approves (closes) a returned loan. Requires `ManageLoans`;
/// a plain member is forbidden.
pub struct ApproveLoan {
    loans: Arc<dyn LoanRepository>,
    clock: Arc<dyn Clock>,
}

impl ApproveLoan {
    pub fn new(loans: Arc<dyn LoanRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { loans, clock }
    }

    pub async fn execute(
        &self,
        actor: &AuthPrincipal,
        loan_id: Uuid,
    ) -> Result<Loan, LendingError> {
        if !actor.role.grants(Permission::ManageLoans) {
            return Err(LendingError::Forbidden);
        }

        let mut loan = self
            .loans
            .find_by_id(loan_id)
            .await?
            .ok_or(LendingError::LoanNotFound)?;

        loan.approve(actor.user_id, self.clock.now())?;
        self.loans.update(loan.clone()).await?;
        Ok(loan)
    }
}
