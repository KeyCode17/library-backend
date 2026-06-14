//! Presentation layer: HTTP DTOs and the catalog router (a driving adapter).

pub mod dto;
pub mod http;

pub use http::{router, CatalogState};
