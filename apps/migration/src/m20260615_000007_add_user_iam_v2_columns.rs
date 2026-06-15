//! Additively add the IAM v2 user columns (`verified`/`active`/`created_at`).
//!
//! On a fresh DB the create-users migration (entity-derived) already adds these,
//! so `ADD COLUMN IF NOT EXISTS` is a no-op. On a 1.0.0→1.1.0 upgrade — where the
//! create-users migration ran before those columns existed on the entity — this
//! is what actually adds them. Idempotent either way. Typed SeaORM DDL (not raw
//! SQL), so the generate-migrations rule is respected.

use sea_orm_migration::prelude::*;

use persistence::entity::user;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(user::Entity)
                    .add_column_if_not_exists(
                        ColumnDef::new(user::Column::Verified)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(user::Entity)
                    .add_column_if_not_exists(
                        ColumnDef::new(user::Column::Active)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(user::Entity)
                    .add_column_if_not_exists(
                        ColumnDef::new(user::Column::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for column in [
            user::Column::CreatedAt,
            user::Column::Active,
            user::Column::Verified,
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(user::Entity)
                        .drop_column(column)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}
