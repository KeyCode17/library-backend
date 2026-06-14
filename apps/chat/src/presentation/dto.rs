//! Wire DTOs, shared by REST history and the WebSocket stream. Mirrors the
//! `ChatMessage` / `ChatSend` / `ChatMessageList` contract schemas.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{ChatMessage, Page};

/// `ChatMessage` — the stored/broadcast shape (server → client).
#[derive(Debug, Clone, Serialize)]
pub struct ChatMessageDto {
    pub id: Uuid,
    pub room: String,
    pub user_id: Uuid,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

impl From<ChatMessage> for ChatMessageDto {
    fn from(message: ChatMessage) -> Self {
        Self {
            id: message.id,
            room: message.room,
            user_id: message.user_id,
            body: message.body,
            created_at: message.created_at,
        }
    }
}

/// `ChatSend` — the client → server send shape over the socket.
#[derive(Debug, Deserialize)]
pub struct ChatSendDto {
    pub body: String,
}

/// `Pagination` schema.
#[derive(Debug, Serialize)]
pub struct PaginationDto {
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub total_pages: u32,
}

/// `ChatMessageList` — paginated history envelope.
#[derive(Debug, Serialize)]
pub struct ChatMessageListResponse {
    pub data: Vec<ChatMessageDto>,
    pub pagination: PaginationDto,
}

impl From<Page<ChatMessage>> for ChatMessageListResponse {
    fn from(page: Page<ChatMessage>) -> Self {
        let pagination = PaginationDto {
            page: page.page,
            page_size: page.page_size,
            total: page.total,
            total_pages: page.total_pages(),
        };
        let data = page.items.into_iter().map(ChatMessageDto::from).collect();
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
