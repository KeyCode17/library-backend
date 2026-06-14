use std::sync::Arc;

use uuid::Uuid;

use crate::domain::{ChatError, ChatMessage, Clock, MessageBroadcaster, MessageRepository};

/// Use case: post a message to a room. Persists it to history, then broadcasts it
/// to the room's live subscribers — so history and delivery never diverge.
pub struct PostMessage {
    messages: Arc<dyn MessageRepository>,
    broadcaster: Arc<dyn MessageBroadcaster>,
    clock: Arc<dyn Clock>,
}

impl PostMessage {
    pub fn new(
        messages: Arc<dyn MessageRepository>,
        broadcaster: Arc<dyn MessageBroadcaster>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            messages,
            broadcaster,
            clock,
        }
    }

    pub async fn execute(
        &self,
        room: String,
        user_id: Uuid,
        body: String,
    ) -> Result<ChatMessage, ChatError> {
        let body = body.trim().to_owned();
        if body.is_empty() {
            return Err(ChatError::EmptyBody);
        }

        let message = ChatMessage {
            id: Uuid::new_v4(),
            room,
            user_id,
            body,
            created_at: self.clock.now(),
        };

        self.messages.insert(message.clone()).await?;
        self.broadcaster.publish(&message);
        Ok(message)
    }
}
