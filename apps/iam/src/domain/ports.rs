//! Secondary ports the application depends on. Infrastructure provides the
//! adapters (Postgres/in-memory store, Argon2, JWT).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::email_token::EmailToken;
use super::error::IamError;
use super::pagination::{Page, PageRequest};
use super::principal::AuthPrincipal;
use super::role::Role;
use super::user::User;

/// Persistence port for users.
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, IamError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, IamError>;
    /// Insert a new user. Returns `EmailAlreadyExists` if the email is taken.
    async fn insert(&self, user: User) -> Result<(), IamError>;
    /// Set a user's role; returns the updated user, or `None` if absent.
    async fn set_role(&self, id: Uuid, role: Role) -> Result<Option<User>, IamError>;
    /// Paginated user list (admin).
    async fn list(&self, request: PageRequest) -> Result<Page<User>, IamError>;
    /// Update a user's email. Returns `EmailAlreadyExists` on conflict, `None` if
    /// the user is absent.
    async fn set_email(&self, id: Uuid, email: &str) -> Result<Option<User>, IamError>;
    /// Activate/deactivate; returns the updated user, or `None` if absent.
    async fn set_active(&self, id: Uuid, active: bool) -> Result<Option<User>, IamError>;
    /// Replace a user's password hash; returns `None` if absent.
    async fn set_password_hash(&self, id: Uuid, hash: &str) -> Result<Option<User>, IamError>;
    /// Mark a user verified; returns `None` if absent.
    async fn set_verified(&self, id: Uuid, verified: bool) -> Result<Option<User>, IamError>;
    /// Delete a user; `true` if a row was removed.
    async fn delete(&self, id: Uuid) -> Result<bool, IamError>;
    /// Count active users holding the admin role (for last-admin lockout safety).
    async fn count_active_admins(&self) -> Result<u64, IamError>;
}

/// Persistence port for single-use email tokens.
#[async_trait]
pub trait EmailTokenRepository: Send + Sync {
    async fn insert(&self, token: EmailToken) -> Result<(), IamError>;
    async fn find_by_hash(&self, token_hash: &str) -> Result<Option<EmailToken>, IamError>;
    /// Mark a token consumed at `at`. Returns `false` if already consumed/absent.
    async fn consume(&self, id: Uuid, at: DateTime<Utc>) -> Result<bool, IamError>;
}

/// Outbound transactional email port. Implemented by the Resend adapter (real,
/// credential-gated) and a fake (dev/tests).
#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn send_verification(&self, email: &str, link: &str) -> Result<(), IamError>;
    async fn send_password_reset(&self, email: &str, link: &str) -> Result<(), IamError>;
}

/// Clock port (for token expiry + timestamps), so use cases are testable.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// Random token generator + matching hash (for at-rest storage and lookup). The
/// raw token goes into the email link; only its hash is persisted.
pub trait TokenGenerator: Send + Sync {
    /// A fresh random token: `(raw, hash)`.
    fn generate(&self) -> (String, String);
    /// Hash a presented raw token the same way, to look up the stored record.
    fn hash(&self, raw: &str) -> String;
}

/// Password hashing port (Argon2 in infrastructure). Synchronous: hashing is
/// CPU-bound, not I/O.
pub trait PasswordHasher: Send + Sync {
    /// Hash a plaintext password into a self-describing PHC string.
    fn hash(&self, plaintext: &str) -> Result<String, IamError>;
    /// Verify a plaintext password against a stored PHC hash. `Ok(false)` means
    /// a clean mismatch; `Err` means the hash could not be processed.
    fn verify(&self, plaintext: &str, phc_hash: &str) -> Result<bool, IamError>;
}

/// A freshly issued token plus its lifetime in seconds.
#[derive(Debug, Clone)]
pub struct IssuedToken {
    pub token: String,
    pub expires_in_secs: i64,
}

/// Token issuing/verification port (JWT HS256 in infrastructure).
pub trait TokenService: Send + Sync {
    /// Issue a signed token carrying the principal's id and role.
    fn issue(&self, principal: &AuthPrincipal) -> Result<IssuedToken, IamError>;
    /// Verify signature and expiry, returning the principal. Any failure maps to
    /// `Unauthorized`.
    fn verify(&self, token: &str) -> Result<AuthPrincipal, IamError>;
}
