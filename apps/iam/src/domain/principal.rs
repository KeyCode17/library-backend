//! The authenticated principal derived from a verified token.

use uuid::Uuid;

use super::role::Role;

/// An authenticated identity: who the caller is and the role they hold. Built
/// only from a token whose signature and expiry have been verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthPrincipal {
    pub user_id: Uuid,
    pub role: Role,
}
