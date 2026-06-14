//! Create the `books` table. DDL is derived from `entity::book` — no raw SQL.

use sea_orm::Schema;
use sea_orm_migration::prelude::*;

use crate::entity::book;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let schema = Schema::new(manager.get_database_backend());
        let table = schema.create_table_from_entity(book::Entity);
        manager.create_table(table).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(book::Entity).to_owned())
            .await
    }
}
