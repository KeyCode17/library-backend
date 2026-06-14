//! Infrastructure layer: adapters implementing domain ports.

pub mod in_memory;
pub mod seaorm;
pub mod seed;

pub use seaorm::SeaOrmBookRepository;
