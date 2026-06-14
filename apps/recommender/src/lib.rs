//! Recommender: a pure, synchronous decision-tree ranking crate (ADR 0005).
//!
//! No I/O, no HTTP, no FFI, no async — just inputs in, ranked ids out. The
//! backend imports this directly for `POST /recommend`; `recommender-ffi` wraps
//! it with UniFFI for on-device (Android/iOS) use. Keeping the logic here, once,
//! means the server and the phone rank identically.

pub mod domain;
pub mod ranking;

pub use domain::{CandidateBook, Preferences};
pub use ranking::Recommender;
