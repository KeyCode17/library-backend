//! Book-finder filter: narrows a listing by physical location and/or ISBN.

use super::book::Book;

/// Optional shelf/row/ISBN criteria. An absent field matches any value, so an
/// empty filter matches every book. Present fields combine with AND.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BookFilter {
    /// Exact shelf label to match, if any.
    pub shelf: Option<String>,
    /// Exact row to match, if any.
    pub row: Option<i32>,
    /// Exact ISBN to match, if any (enables server-side barcode resolution).
    pub isbn: Option<String>,
}

impl BookFilter {
    /// True when `book` satisfies every present criterion.
    pub fn matches(&self, book: &Book) -> bool {
        self.shelf.as_ref().is_none_or(|shelf| &book.shelf == shelf)
            && self.row.is_none_or(|row| book.row == row)
            && self.isbn.as_ref().is_none_or(|isbn| &book.isbn == isbn)
    }

    /// True when no criteria are set (matches everything).
    pub fn is_empty(&self) -> bool {
        self.shelf.is_none() && self.row.is_none() && self.isbn.is_none()
    }
}
