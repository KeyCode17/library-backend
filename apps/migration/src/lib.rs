//! Database migrations for library-backend (SeaORM + Postgres, per ADR 0003).
//!
//! Per the `generate-migrations` rule, the schema is the source of truth and the
//! ORM emits the DDL: each migration calls `Schema::create_table_from_entity`,
//! deriving `CREATE TABLE` from the SeaORM entity rather than hand-written SQL.
//! To change the schema, edit the entity (`entity::book`) and let SeaORM
//! regenerate the statement — never hand-edit emitted SQL.
//!
//! No Rust ORM offers Prisma/Drizzle-style declarative-schema *auto-diff*; this
//! entity-derived approach is the faithful equivalent within the FSD-mandated
//! SeaORM stack.

mod m20260615_000001_create_books_table;
mod m20260615_000002_create_users_table;
mod m20260615_000003_create_loans_table;
mod m20260615_000004_create_chat_messages_table;
mod m20260615_000005_create_notification_tables;
mod m20260615_000006_create_email_tokens_table;
mod m20260615_000007_add_user_iam_v2_columns;

pub use sea_orm_migration::prelude::*;

/// Migration registry consumed by `sea-orm-cli migrate` / a future migrate runner.
pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260615_000001_create_books_table::Migration),
            Box::new(m20260615_000002_create_users_table::Migration),
            Box::new(m20260615_000003_create_loans_table::Migration),
            Box::new(m20260615_000004_create_chat_messages_table::Migration),
            Box::new(m20260615_000005_create_notification_tables::Migration),
            Box::new(m20260615_000006_create_email_tokens_table::Migration),
            Box::new(m20260615_000007_add_user_iam_v2_columns::Migration),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_all_table_migrations() {
        assert_eq!(Migrator::migrations().len(), 7);
    }
}
