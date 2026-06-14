//! Presentation layer: device registration + notification history (both bearer).

pub mod dto;
pub mod http;

pub use http::{router, NotificationState};
