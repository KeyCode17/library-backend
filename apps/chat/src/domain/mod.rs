//! Domain layer: the message entity, ports, pagination, and errors. Pure.

pub mod error;
pub mod message;
pub mod pagination;
pub mod ports;

pub use error::ChatError;
pub use message::ChatMessage;
pub use pagination::{Page, PageRequest};
pub use ports::{Clock, MessageBroadcaster, MessageRepository};
