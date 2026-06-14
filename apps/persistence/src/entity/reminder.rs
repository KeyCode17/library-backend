//! `reminders` table schema. Source of truth for the migration DDL.
//!
//! Mirrors the notification domain `Reminder`. `kind` is the wire kind string
//! (`due_soon` / `overdue`).

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "reminders")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub loan_id: Uuid,
    pub kind: String,
    #[sea_orm(column_type = "Text")]
    pub message: String,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
