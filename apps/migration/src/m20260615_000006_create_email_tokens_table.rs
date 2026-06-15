//! Create the `email_tokens` table. DDL derived from `entity::email_token`.
//!
//! The `users` table's new `verified`/`active`/`created_at` columns are picked up
//! by the (entity-derived) create-users migration — the project is pre-deploy, so
//! a fresh apply of all migrations yields the current schema.

use sea_orm::Schema;
use sea_orm_migration::prelude::*;

use persistence::entity::email_token;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let schema = Schema::new(manager.get_database_backend());
        manager
            .create_table(schema.create_table_from_entity(email_token::Entity))
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(email_token::Entity).to_owned())
            .await
    }
}
