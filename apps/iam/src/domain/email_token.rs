//! Single-use, expiring email tokens (verification + password reset). Only the
//! hash of the random token is ever persisted.

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailTokenKind {
    VerifyEmail,
    PasswordReset,
}

impl EmailTokenKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EmailTokenKind::VerifyEmail => "verify_email",
            EmailTokenKind::PasswordReset => "password_reset",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "verify_email" => Some(EmailTokenKind::VerifyEmail),
            "password_reset" => Some(EmailTokenKind::PasswordReset),
            _ => None,
        }
    }
}

/// A persisted token record. The plaintext token is never stored — only its
/// `token_hash` (hex SHA-256).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub kind: EmailTokenKind,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl EmailToken {
    /// Usable iff not yet consumed and not past expiry at `now`.
    pub fn is_usable(&self, now: DateTime<Utc>) -> bool {
        self.consumed_at.is_none() && self.expires_at > now
    }
}
