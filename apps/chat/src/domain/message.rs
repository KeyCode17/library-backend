//! The `ChatMessage` entity.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// One posted message. `room` is a free-form key (an event id, a book category,
/// or "ask-a-librarian", per PRD §3). `Clone`/`Eq` so it can be broadcast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub id: Uuid,
    pub room: String,
    pub user_id: Uuid,
    pub body: String,
    pub created_at: DateTime<Utc>,
}
