//! Domain layer: entities, value objects, and ports. Pure — no framework deps.

pub mod book;
pub mod error;
pub mod pagination;
pub mod repository;

pub use book::Book;
pub use error::RepositoryError;
pub use pagination::{Page, PageRequest};
pub use repository::BookRepository;
