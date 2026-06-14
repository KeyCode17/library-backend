//! The slice of a loan the scheduler needs. Comes from the `LoanSource` port so
//! the notification domain never depends on `lending`.

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueLoan {
    pub loan_id: Uuid,
    pub user_id: Uuid,
    pub due_at: DateTime<Utc>,
}
