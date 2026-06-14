//! Loan lifecycle status.

use serde::{Deserialize, Serialize};

/// Where a loan is in its lifecycle: `borrowed` → `returned` → `approved`.
/// "Due"/overdue is derived from `due_at`, not a stored status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoanStatus {
    /// Active: the book is out with the borrower.
    Borrowed,
    /// The borrower has returned the book; awaiting staff approval.
    Returned,
    /// Staff has confirmed the return; the loan is closed.
    Approved,
}

impl LoanStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            LoanStatus::Borrowed => "borrowed",
            LoanStatus::Returned => "returned",
            LoanStatus::Approved => "approved",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "borrowed" => Some(LoanStatus::Borrowed),
            "returned" => Some(LoanStatus::Returned),
            "approved" => Some(LoanStatus::Approved),
            _ => None,
        }
    }
}
