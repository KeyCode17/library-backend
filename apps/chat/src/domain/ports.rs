//! Ports the chat use cases depend on.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::error::ChatError;
use super::message::ChatMessage;
use super::pagination::{Page, PageRequest};

/// Persistence port for chat history.
#[async_trait]
pub trait MessageRepository: Send + Sync {
    async fn insert(&self, message: ChatMessage) -> Result<(), ChatError>;
    /// History for a room, oldest first.
    async fn list_by_room(
        &self,
        room: &str,
        request: PageRequest,
    ) -> Result<Page<ChatMessage>, ChatError>;
}

/// Live-delivery port: publish a message to a room's current subscribers. The
/// infrastructure adapter (the room hub) also exposes `subscribe`, used directly
/// by the WebSocket handler.
pub trait MessageBroadcaster: Send + Sync {
    fn publish(&self, message: &ChatMessage);
}

/// Clock port, so use cases are not bound to the wall clock.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}
