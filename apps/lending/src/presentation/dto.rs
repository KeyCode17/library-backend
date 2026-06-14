//! Wire DTOs. Mirrors the contract `Loan` / `LoanList` schemas. Timestamps
//! serialize as RFC 3339 (chrono).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{Loan, LoanStatus, Page};

/// `POST /loans` body.
#[derive(Debug, Deserialize)]
pub struct BorrowRequest {
    pub book_id: Uuid,
}

/// `Loan` schema. Nullable fields are `None` until the lifecycle reaches them.
#[derive(Debug, Serialize)]
pub struct LoanDto {
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

impl From<Loan> for LoanDto {
    fn from(loan: Loan) -> Self {
        Self {
            id: loan.id,
            book_id: loan.book_id,
            user_id: loan.user_id,
            status: loan.status,
            borrowed_at: loan.borrowed_at,
            due_at: loan.due_at,
            returned_at: loan.returned_at,
            approved_by: loan.approved_by,
            approved_at: loan.approved_at,
        }
    }
}

/// `Pagination` schema.
#[derive(Debug, Serialize)]
pub struct PaginationDto {
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub total_pages: u32,
}

/// `LoanList` envelope.
#[derive(Debug, Serialize)]
pub struct LoanListResponse {
    pub data: Vec<LoanDto>,
    pub pagination: PaginationDto,
}

impl From<Page<Loan>> for LoanListResponse {
    fn from(page: Page<Loan>) -> Self {
        let pagination = PaginationDto {
            page: page.page,
            page_size: page.page_size,
            total: page.total,
            total_pages: page.total_pages(),
        };
        let data = page.items.into_iter().map(LoanDto::from).collect();
        Self { data, pagination }
    }
}

/// Shared `Error { code, message }` body.
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: &'static str,
}

impl ErrorBody {
    pub const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}
