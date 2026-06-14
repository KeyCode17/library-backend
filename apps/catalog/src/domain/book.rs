use uuid::Uuid;

/// A catalogued book and its physical location on the shelves.
///
/// Field names and types are bound by the API contract (`contract/openapi.yaml`):
/// the persistence schema (see the `migration` crate) and the presentation DTO
/// both mirror this shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Book {
    pub id: Uuid,
    pub title: String,
    pub author: String,
    pub isbn: String,
    /// Shelf label, e.g. "Tech" or "Fiction".
    pub shelf: String,
    /// Row within the shelf (1-based).
    pub row: i32,
    /// Whether a copy is currently available to borrow.
    pub available: bool,
}
