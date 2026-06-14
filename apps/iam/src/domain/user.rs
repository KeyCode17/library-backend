//! The `User` entity.

use uuid::Uuid;

use super::role::Role;

/// A registered user. `password_hash` is an Argon2 PHC string — never the
/// plaintext password. The presentation layer never serializes this field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub role: Role,
}

impl User {
    pub fn new(id: Uuid, email: String, password_hash: String, role: Role) -> Self {
        Self {
            id,
            email,
            password_hash,
            role,
        }
    }
}
