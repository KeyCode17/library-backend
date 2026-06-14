use std::sync::Arc;

use crate::domain::{ChatError, ChatMessage, MessageRepository, Page, PageRequest};

/// Use case: read a room's message history.
pub struct ListHistory {
    messages: Arc<dyn MessageRepository>,
}

impl ListHistory {
    pub fn new(messages: Arc<dyn MessageRepository>) -> Self {
        Self { messages }
    }

    pub async fn execute(
        &self,
        room: &str,
        request: PageRequest,
    ) -> Result<Page<ChatMessage>, ChatError> {
        self.messages.list_by_room(room, request).await
    }
}
