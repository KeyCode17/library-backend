//! Wire DTOs for IAM. The password hash is never part of any response DTO; all
//! request bodies use `deny_unknown_fields` (defense-in-depth).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{IssuedToken, Page, Role, User};

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

/// `Principal` response: the current user without the password hash.
#[derive(Debug, Serialize)]
pub struct PrincipalResponse {
    pub id: Uuid,
    pub email: String,
    pub role: Role,
    pub verified: bool,
    pub active: bool,
}

impl From<User> for PrincipalResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            role: user.role,
            verified: user.verified,
            active: user.active,
        }
    }
}

/// `UserSummary` — admin view of a user (no password hash).
#[derive(Debug, Serialize)]
pub struct UserSummary {
    pub id: Uuid,
    pub email: String,
    pub role: Role,
    pub verified: bool,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserSummary {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            role: user.role,
            verified: user.verified,
            active: user.active,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PaginationDto {
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub total_pages: u32,
}

/// `UserList` envelope.
#[derive(Debug, Serialize)]
pub struct UserListResponse {
    pub data: Vec<UserSummary>,
    pub pagination: PaginationDto,
}

impl From<Page<User>> for UserListResponse {
    fn from(page: Page<User>) -> Self {
        let pagination = PaginationDto {
            page: page.page,
            page_size: page.page_size,
            total: page.total,
            total_pages: page.total_pages(),
        };
        let data = page.items.into_iter().map(UserSummary::from).collect();
        Self { data, pagination }
    }
}

/// `AssignRoleRequest` body.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssignRoleRequest {
    pub role: Role,
}

/// `CreateUserRequest` body (admin).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub role: Role,
}

/// `UpdateUserRequest` body (admin): any subset.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateUserRequest {
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub active: Option<bool>,
}

/// `ChangePasswordRequest` body.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// `UpdateMeRequest` body.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateMeRequest {
    pub email: String,
}

/// `VerifyEmailRequest` body.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyEmailRequest {
    pub token: String,
}

/// `ForgotPasswordRequest` body.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

/// `ResetPasswordRequest` body.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

/// Shared `Error { code, message }` body.
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
