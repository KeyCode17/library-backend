//! Book-finder filter: narrows a listing by location, ISBN, and/or free-text.

use super::book::Book;

/// Optional shelf/row/ISBN/text criteria. An absent field matches any value, so
/// an empty filter matches every book. Present fields combine with AND; the `query`
/// is a case-insensitive substring over title/author/ISBN.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BookFilter {
    /// Exact shelf label to match, if any.
    pub shelf: Option<String>,
    /// Exact row to match, if any.
    pub row: Option<i32>,
    /// Exact ISBN to match, if any (server-side barcode resolution).
    pub isbn: Option<String>,
    /// Free-text search: case-insensitive substring over title, author, and ISBN.
    pub query: Option<String>,
}

impl BookFilter {
    /// True when `book` satisfies every present criterion.
    pub fn matches(&self, book: &Book) -> bool {
        self.shelf.as_ref().is_none_or(|shelf| &book.shelf == shelf)
            && self.row.is_none_or(|row| book.row == row)
            && self.isbn.as_ref().is_none_or(|isbn| &book.isbn == isbn)
            && self
                .query
                .as_ref()
                .is_none_or(|query| matches_text(book, query))
    }

    /// True when no criteria are set (matches everything).
    pub fn is_empty(&self) -> bool {
        self.shelf.is_none() && self.row.is_none() && self.isbn.is_none() && self.query.is_none()
    }
}

/// Case-insensitive substring of `query` in the book's title, author, or ISBN.
fn matches_text(book: &Book, query: &str) -> bool {
    let needle = query.to_lowercase();
    book.title.to_lowercase().contains(&needle)
        || book.author.to_lowercase().contains(&needle)
        || book.isbn.to_lowercase().contains(&needle)
}
