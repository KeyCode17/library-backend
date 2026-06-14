//! Bridges the lending `BookGateway` port to the catalog `BookRepository`.
//!
//! Lives in the composition root, not in either context, so `catalog` and
//! `lending` stay decoupled at the domain level (ADR 0002). It is the one place
//! that knows both sides.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use catalog::domain::{BookRepository, ClaimOutcome as CatalogClaim};
use lending::domain::{BookGateway, ClaimOutcome as LendingClaim, LendingError};

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
    async fn claim_for_borrow(&self, book_id: Uuid) -> Result<LendingClaim, LendingError> {
        match self.books.claim_if_available(book_id).await {
            Ok(CatalogClaim::Claimed) => Ok(LendingClaim::Claimed),
            Ok(CatalogClaim::Unavailable) => Ok(LendingClaim::Unavailable),
            Ok(CatalogClaim::NotFound) => Ok(LendingClaim::NotFound),
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
