//! Wire DTOs for IAM. Mirrors the contract schemas. The password hash is never
//! part of any response DTO.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{IssuedToken, Role, User};

/// `Credentials` request body for register and login.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialsRequest {
    pub email: String,
    pub password: String,
}

/// `AuthToken` response from login.
#[derive(Debug, Serialize)]
pub struct AuthTokenResponse {
    pub token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
}

impl From<IssuedToken> for AuthTokenResponse {
    fn from(issued: IssuedToken) -> Self {
        Self {
            token: issued.token,
            token_type: "Bearer",
            expires_in: issued.expires_in_secs,
        }
    }
}

/// `Principal` response: a user record without the password hash.
#[derive(Debug, Serialize)]
pub struct PrincipalResponse {
    pub id: Uuid,
    pub email: String,
    pub role: Role,
}

impl From<User> for PrincipalResponse {
    fn from(user: User) -> Self {
        // Deliberately drops `password_hash` — it must never be serialized.
        Self {
            id: user.id,
            email: user.email,
            role: user.role,
        }
    }
}

/// `AssignRoleRequest` body for the admin role-management endpoint.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssignRoleRequest {
    pub role: Role,
}

/// Shared `Error { code, message }` body (matches the contract's `Error`).
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: &'static str,
}

impl ErrorBody {
    pub const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}
