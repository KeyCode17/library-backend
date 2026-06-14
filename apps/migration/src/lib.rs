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

pub mod entity;
mod m20260615_000001_create_books_table;
mod m20260615_000002_create_users_table;
mod m20260615_000003_create_loans_table;

pub use sea_orm_migration::prelude::*;

/// Migration registry consumed by `sea-orm-cli migrate` / a future migrate runner.
pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260615_000001_create_books_table::Migration),
            Box::new(m20260615_000002_create_users_table::Migration),
            Box::new(m20260615_000003_create_loans_table::Migration),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_the_books_users_and_loans_migrations() {
        assert_eq!(Migrator::migrations().len(), 3);
    }
}
