//! Chat bounded context: group chat over WebSocket (ADR 0006).
//!
//! Persistence follows the skill (history via a `MessageRepository`); delivery is
//! WebSocket + a broadcast/connection registry (`RoomHub`), not the REST template.
//! Auth is the IAM JWT.
//!
//! Hexagonal layering (ADR 0002):
//! - `domain` — `ChatMessage`, the `MessageRepository` / `MessageBroadcaster` /
//!   `Clock` ports, pagination, errors. Pure.
//! - `application` — post-message (persist + broadcast) and list-history use cases.
//! - `infrastructure` — in-memory message store, the room hub, the system clock.
//! - `presentation` — REST history (bearer) and the WebSocket endpoint.

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;
