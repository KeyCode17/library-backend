//! The `Loan` entity. State transitions live here so they cannot be bypassed.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::error::LendingError;
use super::status::LoanStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loan {
    pub id: Uuid,
    pub book_id: Uuid,
    pub user_id: Uuid,
    pub status: LoanStatus,
    pub borrowed_at: DateTime<Utc>,
    pub due_at: DateTime<Utc>,
    pub returned_at: Option<DateTime<Utc>>,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
}

impl Loan {
    /// Open a new active loan.
    pub fn borrow(
        id: Uuid,
        book_id: Uuid,
        user_id: Uuid,
        borrowed_at: DateTime<Utc>,
        due_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            book_id,
            user_id,
            status: LoanStatus::Borrowed,
            borrowed_at,
            due_at,
            returned_at: None,
            approved_by: None,
            approved_at: None,
        }
    }

    /// `Borrowed` → `Returned`. Errors with `InvalidState` otherwise.
    pub fn mark_returned(&mut self, at: DateTime<Utc>) -> Result<(), LendingError> {
        if self.status != LoanStatus::Borrowed {
            return Err(LendingError::InvalidState);
        }
        self.status = LoanStatus::Returned;
        self.returned_at = Some(at);
        Ok(())
    }

    /// `Returned` → `Approved` (staff confirms the return). `InvalidState`
    /// otherwise.
    pub fn approve(&mut self, approver: Uuid, at: DateTime<Utc>) -> Result<(), LendingError> {
        if self.status != LoanStatus::Returned {
            return Err(LendingError::InvalidState);
        }
        self.status = LoanStatus::Approved;
        self.approved_by = Some(approver);
        self.approved_at = Some(at);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Loan {
        let now = DateTime::from_timestamp(1_700_000_000, 0).expect("timestamp");
        Loan::borrow(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), now, now)
    }

    #[test]
    fn lifecycle_advances_borrowed_returned_approved() {
        let now = DateTime::from_timestamp(1_700_000_100, 0).expect("timestamp");
        let mut loan = fresh();
        assert_eq!(loan.status, LoanStatus::Borrowed);

        loan.mark_returned(now).expect("borrowed -> returned");
        assert_eq!(loan.status, LoanStatus::Returned);
        assert_eq!(loan.returned_at, Some(now));

        let approver = Uuid::new_v4();
        loan.approve(approver, now).expect("returned -> approved");
        assert_eq!(loan.status, LoanStatus::Approved);
        assert_eq!(loan.approved_by, Some(approver));
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        let now = DateTime::from_timestamp(1_700_000_100, 0).expect("timestamp");
        let mut loan = fresh();

        // Cannot approve before returning.
        assert!(matches!(
            loan.approve(Uuid::new_v4(), now),
            Err(LendingError::InvalidState)
        ));

        // Cannot return twice.
        loan.mark_returned(now).expect("first return");
        assert!(matches!(
            loan.mark_returned(now),
            Err(LendingError::InvalidState)
        ));
    }
}
