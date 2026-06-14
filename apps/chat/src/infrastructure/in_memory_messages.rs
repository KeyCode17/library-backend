//! In-memory `MessageRepository`. Stand-in until the Postgres/SeaORM adapter is
//! wired (the chat_messages table schema lives in the `migration` crate).

use std::sync::RwLock;

use async_trait::async_trait;

use crate::domain::{ChatError, ChatMessage, MessageRepository, Page, PageRequest};

pub struct InMemoryMessageRepository {
    messages: RwLock<Vec<ChatMessage>>,
}

impl InMemoryMessageRepository {
    pub fn new() -> Self {
        Self {
            messages: RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryMessageRepository {
    fn default() -> Self {
        Self::new()
    }
}

fn poisoned() -> ChatError {
    ChatError::Repository("message store lock poisoned".to_owned())
}

#[async_trait]
impl MessageRepository for InMemoryMessageRepository {
    async fn insert(&self, message: ChatMessage) -> Result<(), ChatError> {
        let mut guard = self.messages.write().map_err(|_| poisoned())?;
        guard.push(message);
        Ok(())
    }

    async fn list_by_room(
        &self,
        room: &str,
        request: PageRequest,
    ) -> Result<Page<ChatMessage>, ChatError> {
        // Insertion order is chronological (oldest first).
        let in_room: Vec<ChatMessage> = {
            let guard = self.messages.read().map_err(|_| poisoned())?;
            guard
                .iter()
                .filter(|message| message.room == room)
                .cloned()
                .collect()
        };

        let total = in_room.len() as u64;
        let offset = request.offset() as usize;
        let items = in_room
            .into_iter()
            .skip(offset)
            .take(request.page_size() as usize)
            .collect();

        Ok(Page {
            items,
            page: request.page(),
            page_size: request.page_size(),
            total,
        })
    }
}
