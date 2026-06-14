//! FFI DTOs. IDs cross the boundary as `String` (UniFFI has no native UUID).

/// User preferences passed from the mobile side.
#[derive(uniffi::Record)]
pub struct PreferencesDto {
    pub preferred_shelves: Vec<String>,
    pub preferred_authors: Vec<String>,
    pub available_only: bool,
}

/// A candidate book to rank. `id` is a uuid string.
#[derive(uniffi::Record)]
pub struct CandidateBookDto {
    pub id: String,
    pub shelf: String,
    pub author: String,
    pub available: bool,
}
