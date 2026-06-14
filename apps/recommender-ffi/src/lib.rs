//! UniFFI wrapper over the pure `recommender` crate (ADR 0005).
//!
//! Thin by design: it maps FFI-shaped DTOs to the pure types, calls
//! `recommender`, and maps the result back. No ranking logic lives here — the
//! server and the phone share the one implementation in `recommender`.

uniffi::setup_scaffolding!();

mod dto;
mod engine;
mod error;

pub use dto::{CandidateBookDto, PreferencesDto};
pub use engine::RecommenderEngine;
pub use error::FfiError;
