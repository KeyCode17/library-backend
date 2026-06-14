//! Inputs to the ranking. Plain value types — no framework, no FFI derives.

use uuid::Uuid;

/// What a user likes. Empty vectors mean "no preference on that axis".
#[derive(Debug, Clone, Default)]
pub struct Preferences {
    /// Shelves/genres the user favours (matched case-insensitively).
    pub preferred_shelves: Vec<String>,
    /// Authors the user favours (matched case-insensitively).
    pub preferred_authors: Vec<String>,
    /// When true, unavailable books are dropped from the ranking entirely.
    pub available_only: bool,
}

/// A book the recommender may rank. The minimal projection the ranking needs —
/// the caller supplies these (from the request body or the server catalog).
#[derive(Debug, Clone)]
pub struct CandidateBook {
    pub id: Uuid,
    pub shelf: String,
    pub author: String,
    pub available: bool,
}
