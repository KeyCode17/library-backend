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

impl Platform {
    pub fn as_str(self) -> &'static str {
        match self {
            Platform::Android => "android",
            Platform::Ios => "ios",
            Platform::Web => "web",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "android" => Some(Platform::Android),
            "ios" => Some(Platform::Ios),
            "web" => Some(Platform::Web),
            _ => None,
        }
    }
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
