use std::sync::Arc;

use crate::domain::{Book, BookFilter, BookRepository, Page, PageRequest, RepositoryError};

/// Use case: return a page of books from the catalog, optionally narrowed by a
/// shelf/row finder filter.
///
/// Depends only on the `BookRepository` port, so it is unit-testable with a fake
/// repository (ADR 0002) and agnostic to the storage backend.
pub struct ListBooks {
    repository: Arc<dyn BookRepository>,
}

impl ListBooks {
    pub fn new(repository: Arc<dyn BookRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        filter: &BookFilter,
        request: PageRequest,
    ) -> Result<Page<Book>, RepositoryError> {
        self.repository.list(filter, request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::in_memory::InMemoryBookRepository;

    fn use_case() -> ListBooks {
        let repository: Arc<dyn BookRepository> = Arc::new(InMemoryBookRepository::seeded());
        ListBooks::new(repository)
    }

    #[tokio::test]
    async fn lists_all_seeded_books_on_a_large_first_page() {
        let page = use_case()
            .execute(&BookFilter::default(), PageRequest::new(1, 20))
            .await
            .expect("listing succeeds");

        assert_eq!(page.total, 8);
        assert_eq!(page.items.len(), 8);
        assert_eq!(page.page, 1);
        assert_eq!(page.total_pages(), 1);
    }

    #[tokio::test]
    async fn paginates_when_page_size_is_small() {
        let page = use_case()
            .execute(&BookFilter::default(), PageRequest::new(2, 3))
            .await
            .expect("listing succeeds");

        assert_eq!(page.total, 8);
        assert_eq!(page.items.len(), 3);
        assert_eq!(page.page, 2);
        assert_eq!(page.total_pages(), 3);
    }

    #[tokio::test]
    async fn finder_filters_by_shelf() {
        let filter = BookFilter {
            shelf: Some("Tech".to_owned()),
            row: None,
        };
        let page = use_case()
            .execute(&filter, PageRequest::new(1, 20))
            .await
            .expect("listing succeeds");

        assert_eq!(page.total, 3);
        assert!(page.items.iter().all(|book| book.shelf == "Tech"));
    }

    #[tokio::test]
    async fn finder_filters_by_shelf_and_row() {
        let filter = BookFilter {
            shelf: Some("Tech".to_owned()),
            row: Some(4),
        };
        let page = use_case()
            .execute(&filter, PageRequest::new(1, 20))
            .await
            .expect("listing succeeds");

        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].title, "The Rust Programming Language");
    }

    #[tokio::test]
    async fn finder_returns_empty_when_nothing_matches() {
        let filter = BookFilter {
            shelf: Some("Nonexistent".to_owned()),
            row: None,
        };
        let page = use_case()
            .execute(&filter, PageRequest::new(1, 20))
            .await
            .expect("listing succeeds");

        assert_eq!(page.total, 0);
        assert!(page.items.is_empty());
    }
}
