//! A registered push device (FCM token) for a user.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The platform a device runs. Android is the consumer of this milestone; the
/// others are kept for the shared schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Android,
    Ios,
    Web,
}

/// A device token registered to a user. The pair (user, token) is unique — a
/// repeat registration of the same token refreshes the entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub platform: Platform,
    pub registered_at: DateTime<Utc>,
}
