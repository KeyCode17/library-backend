use std::sync::Arc;

use uuid::Uuid;

use crate::domain::{Book, BookRepository, RepositoryError};

/// Use case: fetch a single book by id. Returns `Ok(None)` when absent so the
/// presentation layer can map it to a `404`; reserves `Err` for backend failures.
pub struct GetBook {
    repository: Arc<dyn BookRepository>,
}

impl GetBook {
    pub fn new(repository: Arc<dyn BookRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, id: Uuid) -> Result<Option<Book>, RepositoryError> {
        self.repository.find_by_id(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::in_memory::InMemoryBookRepository;
    use uuid::uuid;

    fn use_case() -> GetBook {
        let repository: Arc<dyn BookRepository> = Arc::new(InMemoryBookRepository::seeded());
        GetBook::new(repository)
    }

    #[tokio::test]
    async fn returns_the_book_when_it_exists() {
        let id = uuid!("00000000-0000-4000-8000-000000000002");
        let book = use_case().execute(id).await.expect("lookup succeeds");

        let book = book.expect("book is present");
        assert_eq!(book.id, id);
        assert_eq!(book.title, "Clean Code");
    }

    #[tokio::test]
    async fn returns_none_when_absent() {
        let id = uuid!("ffffffff-ffff-4fff-8fff-ffffffffffff");
        let book = use_case().execute(id).await.expect("lookup succeeds");

        assert!(book.is_none());
    }
}
