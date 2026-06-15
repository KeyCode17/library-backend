//! Partial unique index: at most one active (`borrowed`) loan per book — a DB
//! backstop for the application-level borrow atomicity (the conditional book
//! claim).
//!
//! SeaORM's schema builder cannot express a partial (`WHERE`) index, so this is
//! the one **hand-written SQL** migration — an authorized, narrowly-scoped
//! exception to the generate-migrations rule, documented in ADR-0007.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX IF NOT EXISTS uniq_active_loan_per_book \
                 ON loans (book_id) WHERE status = 'borrowed'",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS uniq_active_loan_per_book")
            .await?;
        Ok(())
    }
}
