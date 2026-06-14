//! Postgres/SeaORM `MessageRepository` adapter.

use async_trait::async_trait;

use persistence::entity::chat_message;
use persistence::sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

use crate::domain::{ChatError, ChatMessage, MessageRepository, Page, PageRequest};

pub struct SeaOrmMessageRepository {
    db: DatabaseConnection,
}

impl SeaOrmMessageRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn backend<E: std::fmt::Display>(error: E) -> ChatError {
    ChatError::Repository(error.to_string())
}

fn to_domain(model: chat_message::Model) -> ChatMessage {
    ChatMessage {
        id: model.id,
        room: model.room,
        user_id: model.user_id,
        body: model.body,
        created_at: model.created_at,
    }
}

#[async_trait]
impl MessageRepository for SeaOrmMessageRepository {
    async fn insert(&self, message: ChatMessage) -> Result<(), ChatError> {
        let active = chat_message::ActiveModel {
            id: Set(message.id),
            room: Set(message.room),
            user_id: Set(message.user_id),
            body: Set(message.body),
            created_at: Set(message.created_at),
        };
        active.insert(&self.db).await.map_err(backend)?;
        Ok(())
    }

    async fn list_by_room(
        &self,
        room: &str,
        request: PageRequest,
    ) -> Result<Page<ChatMessage>, ChatError> {
        let paginator = chat_message::Entity::find()
            .filter(chat_message::Column::Room.eq(room))
            .order_by_asc(chat_message::Column::CreatedAt)
            .order_by_asc(chat_message::Column::Id)
            .paginate(&self.db, u64::from(request.page_size()));
        let total = paginator.num_items().await.map_err(backend)?;
        let models = paginator
            .fetch_page(u64::from(request.page() - 1))
            .await
            .map_err(backend)?;

        Ok(Page {
            items: models.into_iter().map(to_domain).collect(),
            page: request.page(),
            page_size: request.page_size(),
            total,
        })
    }
}
