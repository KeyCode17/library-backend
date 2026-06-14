//! The broadcast/connection registry (ADR 0006).
//!
//! One `tokio::sync::broadcast` channel per room. The WebSocket handler
//! `subscribe`s to receive a room's live messages; the post-message use case
//! `publish`es through the `MessageBroadcaster` impl. Empty rooms keep their
//! channel (cheap) — fine at this scale.

use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::broadcast;

use crate::domain::{ChatMessage, MessageBroadcaster};

const CHANNEL_CAPACITY: usize = 256;

pub struct RoomHub {
    rooms: Mutex<HashMap<String, broadcast::Sender<ChatMessage>>>,
}

impl RoomHub {
    pub fn new() -> Self {
        Self {
            rooms: Mutex::new(HashMap::new()),
        }
    }

    /// Subscribe to a room's live messages, creating the channel on first use.
    pub fn subscribe(&self, room: &str) -> broadcast::Receiver<ChatMessage> {
        let mut rooms = self.rooms.lock().expect("room hub lock");
        rooms
            .entry(room.to_owned())
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0)
            .subscribe()
    }

    fn sender_for(&self, room: &str) -> broadcast::Sender<ChatMessage> {
        let mut rooms = self.rooms.lock().expect("room hub lock");
        rooms
            .entry(room.to_owned())
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0)
            .clone()
    }
}

impl Default for RoomHub {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageBroadcaster for RoomHub {
    fn publish(&self, message: &ChatMessage) {
        // `send` errors only when there are no receivers; that is fine.
        let _ = self.sender_for(&message.room).send(message.clone());
    }
}
