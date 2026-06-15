//! The `User` entity.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::role::Role;

/// A registered user. `password_hash` is an Argon2 PHC string — never the
/// plaintext password. The presentation layer never serializes that field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub role: Role,
    /// Whether the email has been verified (login is not gated on this).
    pub verified: bool,
    /// Whether the account is active; deactivated accounts cannot log in.
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

impl User {
    /// A freshly registered user: unverified, active.
    pub fn new(
        id: Uuid,
        email: String,
        password_hash: String,
        role: Role,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            email,
            password_hash,
            role,
            verified: false,
            active: true,
            created_at,
        }
    }
}
