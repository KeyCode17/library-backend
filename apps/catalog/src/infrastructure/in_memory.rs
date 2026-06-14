use async_trait::async_trait;
use uuid::{uuid, Uuid};

use crate::domain::{Book, BookFilter, BookRepository, Page, PageRequest, RepositoryError};

/// In-memory `BookRepository` seeded with a fixed catalog.
///
/// This is the catalog adapter for the T-001 read-only slice; the Postgres/SeaORM
/// adapter lands with the DB wiring (the books table schema lives in the
/// `migration` crate). Deterministic ids keep tests stable. Books are returned in
/// insertion order so pagination is reproducible.
pub struct InMemoryBookRepository {
    books: Vec<Book>,
}

impl InMemoryBookRepository {
    /// Build a repository populated with the seed catalog.
    pub fn seeded() -> Self {
        Self {
            books: seed_books(),
        }
    }
}

#[async_trait]
impl BookRepository for InMemoryBookRepository {
    async fn list(
        &self,
        filter: &BookFilter,
        request: PageRequest,
    ) -> Result<Page<Book>, RepositoryError> {
        let matches: Vec<&Book> = self
            .books
            .iter()
            .filter(|book| filter.matches(book))
            .collect();
        let total = matches.len() as u64;
        let offset = request.offset() as usize;
        let items = matches
            .into_iter()
            .skip(offset)
            .take(request.page_size() as usize)
            .cloned()
            .collect();

        Ok(Page {
            items,
            page: request.page(),
            page_size: request.page_size(),
            total,
        })
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Book>, RepositoryError> {
        Ok(self.books.iter().find(|book| book.id == id).cloned())
    }
}

fn book(
    id: Uuid,
    title: &str,
    author: &str,
    isbn: &str,
    shelf: &str,
    row: i32,
    available: bool,
) -> Book {
    Book {
        id,
        title: title.to_owned(),
        author: author.to_owned(),
        isbn: isbn.to_owned(),
        shelf: shelf.to_owned(),
        row,
        available,
    }
}

/// The seed catalog: eight real books across a few shelves.
fn seed_books() -> Vec<Book> {
    vec![
        book(
            uuid!("00000000-0000-4000-8000-000000000001"),
            "The Pragmatic Programmer",
            "Andrew Hunt and David Thomas",
            "978-0135957059",
            "Tech",
            3,
            true,
        ),
        book(
            uuid!("00000000-0000-4000-8000-000000000002"),
            "Clean Code",
            "Robert C. Martin",
            "978-0132350884",
            "Tech",
            3,
            true,
        ),
        book(
            uuid!("00000000-0000-4000-8000-000000000003"),
            "The Rust Programming Language",
            "Steve Klabnik and Carol Nichols",
            "978-1718503106",
            "Tech",
            4,
            true,
        ),
        book(
            uuid!("00000000-0000-4000-8000-000000000004"),
            "Dune",
            "Frank Herbert",
            "978-0441013593",
            "SciFi",
            1,
            true,
        ),
        book(
            uuid!("00000000-0000-4000-8000-000000000005"),
            "1984",
            "George Orwell",
            "978-0451524935",
            "Fiction",
            2,
            false,
        ),
        book(
            uuid!("00000000-0000-4000-8000-000000000006"),
            "The Hobbit",
            "J.R.R. Tolkien",
            "978-0547928227",
            "Fantasy",
            5,
            true,
        ),
        book(
            uuid!("00000000-0000-4000-8000-000000000007"),
            "Sapiens: A Brief History of Humankind",
            "Yuval Noah Harari",
            "978-0062316097",
            "History",
            2,
            true,
        ),
        book(
            uuid!("00000000-0000-4000-8000-000000000008"),
            "Thinking, Fast and Slow",
            "Daniel Kahneman",
            "978-0374533557",
            "Psychology",
            1,
            true,
        ),
    ]
}
