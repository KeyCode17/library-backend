//! Book-finder filter: narrows a listing by physical location (shelf and/or row).

use super::book::Book;

/// Optional shelf/row criteria. An absent field matches any value, so an empty
/// filter matches every book. Both present means "shelf AND row".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BookFilter {
    /// Exact shelf label to match, if any.
    pub shelf: Option<String>,
    /// Exact row to match, if any.
    pub row: Option<i32>,
}

impl BookFilter {
    /// True when `book` satisfies every present criterion.
    pub fn matches(&self, book: &Book) -> bool {
        self.shelf.as_ref().is_none_or(|shelf| &book.shelf == shelf)
            && self.row.is_none_or(|row| book.row == row)
    }

    /// True when no criteria are set (matches everything).
    pub fn is_empty(&self) -> bool {
        self.shelf.is_none() && self.row.is_none()
    }
}
