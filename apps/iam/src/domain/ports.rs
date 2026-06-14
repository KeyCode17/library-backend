//! Secondary ports the application depends on. Infrastructure provides the
//! adapters (Postgres/in-memory store, Argon2, JWT).

use async_trait::async_trait;
use uuid::Uuid;

use super::error::IamError;
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
