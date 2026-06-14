//! Create the `devices` and `reminders` tables. DDL derived from the entities.

use sea_orm::Schema;
use sea_orm_migration::prelude::*;

use crate::entity::{device, reminder};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let schema = Schema::new(backend);
        manager
            .create_table(schema.create_table_from_entity(device::Entity))
            .await?;
        manager
            .create_table(schema.create_table_from_entity(reminder::Entity))
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(reminder::Entity).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(device::Entity).to_owned())
            .await
    }
}
