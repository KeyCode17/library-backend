//! Create the `users` table. DDL is derived from `entity::user` — no raw SQL.

use sea_orm::Schema;
use sea_orm_migration::prelude::*;

use crate::entity::user;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let schema = Schema::new(manager.get_database_backend());
        let table = schema.create_table_from_entity(user::Entity);
        manager.create_table(table).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(user::Entity).to_owned())
            .await
    }
}
