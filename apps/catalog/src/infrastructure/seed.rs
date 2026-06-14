//! Idempotent Postgres seeding of the catalog (the same seed data the in-memory
//! store uses).

use persistence::entity::book;
use persistence::sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, Set,
};

use crate::domain::RepositoryError;

use super::in_memory::seed_books;

fn backend<E: std::fmt::Display>(error: E) -> RepositoryError {
    RepositoryError::Backend(error.to_string())
}

/// Insert the seed books if the table is empty. Safe to call on every startup.
pub async fn seed_books_if_empty(db: &DatabaseConnection) -> Result<(), RepositoryError> {
    let count = book::Entity::find().count(db).await.map_err(backend)?;
    if count > 0 {
        return Ok(());
    }

    for seed in seed_books() {
        book::ActiveModel {
            id: Set(seed.id),
            title: Set(seed.title),
            author: Set(seed.author),
            isbn: Set(seed.isbn),
            shelf: Set(seed.shelf),
            row: Set(seed.row),
            available: Set(seed.available),
        }
        .insert(db)
        .await
        .map_err(backend)?;
    }
    Ok(())
}
