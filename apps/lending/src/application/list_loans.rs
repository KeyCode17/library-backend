use std::sync::Arc;

use iam::domain::{AuthPrincipal, Permission};

use crate::domain::{LendingError, Loan, LoanRepository, Page, PageRequest};

/// Use case: list loans. Staff (`ManageLoans`) see all loans; a member sees only
/// their own.
pub struct ListLoans {
    loans: Arc<dyn LoanRepository>,
}

impl ListLoans {
    pub fn new(loans: Arc<dyn LoanRepository>) -> Self {
        Self { loans }
    }

    pub async fn execute(
        &self,
        actor: &AuthPrincipal,
        request: PageRequest,
    ) -> Result<Page<Loan>, LendingError> {
        if actor.role.grants(Permission::ManageLoans) {
            self.loans.list_all(request).await
        } else {
            self.loans.list_for_user(actor.user_id, request).await
        }
    }
}
