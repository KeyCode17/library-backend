//! Wire DTOs. Field names and order mirror `contract/openapi.yaml` exactly — the
//! contract is the source of truth, these structs must not drift from it.

use serde::Serialize;
use uuid::Uuid;

use crate::domain::{Book, Page};

/// `Book` schema. `id` serializes as a uuid string via `uuid`'s serde support.
#[derive(Debug, Serialize)]
pub struct BookDto {
    pub id: Uuid,
    pub title: String,
    pub author: String,
    pub isbn: String,
    pub shelf: String,
    pub row: i32,
    pub available: bool,
}

impl From<Book> for BookDto {
    fn from(book: Book) -> Self {
        Self {
            id: book.id,
            title: book.title,
            author: book.author,
            isbn: book.isbn,
            shelf: book.shelf,
            row: book.row,
            available: book.available,
        }
    }
}

/// `Pagination` schema.
#[derive(Debug, Serialize)]
pub struct PaginationDto {
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub total_pages: u32,
}

/// `BookList` schema: the `{ data, pagination }` envelope `GET /books` returns.
#[derive(Debug, Serialize)]
pub struct BookListResponse {
    pub data: Vec<BookDto>,
    pub pagination: PaginationDto,
}

impl From<Page<Book>> for BookListResponse {
    fn from(page: Page<Book>) -> Self {
        let pagination = PaginationDto {
            page: page.page,
            page_size: page.page_size,
            total: page.total,
            total_pages: page.total_pages(),
        };
        let data = page.items.into_iter().map(BookDto::from).collect();
        Self { data, pagination }
    }
}

/// `Error` schema: a flat `{ code, message }` body for 4xx/5xx responses. One
/// shared error shape keeps the contract self-consistent (FSD §4).
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: &'static str,
}

impl ErrorBody {
    pub const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}
