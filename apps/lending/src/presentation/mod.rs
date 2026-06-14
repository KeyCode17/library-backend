//! Presentation layer: HTTP DTOs and the lending router (a driving adapter).

pub mod dto;
pub mod http;

pub use http::{router, LendingState};
