//! Bridges the notification `LoanSource` port to the lending `LoanRepository`.
//!
//! Lives in the composition root so `notification` and `lending` stay decoupled
//! at the domain level (ADR 0002).

use std::sync::Arc;

use async_trait::async_trait;

use lending::domain::LoanRepository;
use notification::domain::{DueLoan, LoanSource, NotificationError};

pub struct LendingLoanSource {
    loans: Arc<dyn LoanRepository>,
}

impl LendingLoanSource {
    pub fn new(loans: Arc<dyn LoanRepository>) -> Self {
        Self { loans }
    }
}

#[async_trait]
impl LoanSource for LendingLoanSource {
    async fn active_loans(&self) -> Result<Vec<DueLoan>, NotificationError> {
        let loans = self
            .loans
            .list_active()
            .await
            .map_err(|_| NotificationError::Dependency("lending".to_owned()))?;

        Ok(loans
            .into_iter()
            .map(|loan| DueLoan {
                loan_id: loan.id,
                user_id: loan.user_id,
                due_at: loan.due_at,
            })
            .collect())
    }
}
