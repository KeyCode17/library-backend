//! Bridges the lending `BookGateway` port to the catalog `BookRepository`.
//!
//! This adapter lives in the composition root, not in either context, so
//! `catalog` and `lending` stay decoupled at the domain level (ADR 0002). It is
//! the one place that knows both sides.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use catalog::domain::BookRepository;
use lending::domain::{BookGateway, LendingError};

pub struct CatalogBookGateway {
    books: Arc<dyn BookRepository>,
}

impl CatalogBookGateway {
    pub fn new(books: Arc<dyn BookRepository>) -> Self {
        Self { books }
    }
}

#[async_trait]
impl BookGateway for CatalogBookGateway {
    async fn is_available(&self, book_id: Uuid) -> Result<Option<bool>, LendingError> {
        match self.books.find_by_id(book_id).await {
            Ok(Some(book)) => Ok(Some(book.available)),
            Ok(None) => Ok(None),
            Err(_) => Err(LendingError::Dependency("catalog".to_owned())),
        }
    }

    async fn set_available(&self, book_id: Uuid, available: bool) -> Result<(), LendingError> {
        match self.books.set_availability(book_id, available).await {
            Ok(Some(_)) => Ok(()),
            Ok(None) => Err(LendingError::BookNotFound),
            Err(_) => Err(LendingError::Dependency("catalog".to_owned())),
        }
    }
}
