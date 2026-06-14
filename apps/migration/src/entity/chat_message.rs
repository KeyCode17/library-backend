//! `chat_messages` table schema. Source of truth for the migration DDL.
//!
//! Mirrors the chat domain `ChatMessage`. `room` is a free-form key; `body` is
//! free text.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "chat_messages")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub room: String,
    pub user_id: Uuid,
    #[sea_orm(column_type = "Text")]
    pub body: String,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
