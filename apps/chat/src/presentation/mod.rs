//! Presentation layer: REST history (bearer) and the WebSocket endpoint.

pub mod dto;
pub mod http;
pub mod ws;

pub use http::{router, ChatState};
