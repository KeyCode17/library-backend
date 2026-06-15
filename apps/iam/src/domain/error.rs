//! IAM domain errors. Variants carry no sensitive data; `Display` is generic so
//! nothing secret reaches logs.

use std::fmt;

#[derive(Debug)]
pub enum IamError {
    /// Registration with an email that is already taken.
    EmailAlreadyExists,
    /// Login failed (unknown email OR wrong password — never distinguished, to
    /// avoid user enumeration).
    InvalidCredentials,
    /// The requested user does not exist.
    UserNotFound,
    /// No valid authentication was presented.
    Unauthorized,
    /// Authenticated, but the role lacks the required permission.
    Forbidden,
    /// Password failed policy (too short).
    WeakPassword,
    /// Email failed validation.
    InvalidEmail,
    /// Hashing/verification backend failure.
    Hashing(String),
    /// Token issuing/verification backend failure.
    Token(String),
    /// An email/verification/reset token was unknown or malformed.
    InvalidToken,
    /// The token has expired.
    TokenExpired,
    /// The token was already used (single-use).
    TokenConsumed,
    /// The action would remove the last admin (or self-lockout) — prevented.
    LastAdmin,
    /// Outbound email failed (best-effort; not surfaced on enumeration-safe paths).
    Email(String),
    /// User store failure.
    Repository(String),
}

impl fmt::Display for IamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            IamError::EmailAlreadyExists => "email already registered",
            IamError::InvalidCredentials => "invalid credentials",
            IamError::UserNotFound => "user not found",
            IamError::Unauthorized => "unauthorized",
            IamError::Forbidden => "forbidden",
            IamError::WeakPassword => "password too short",
            IamError::InvalidEmail => "invalid email",
            IamError::Hashing(_) => "hashing failure",
            IamError::Token(_) => "token failure",
            IamError::InvalidToken => "invalid token",
            IamError::TokenExpired => "token expired",
            IamError::TokenConsumed => "token already used",
            IamError::LastAdmin => "cannot remove the last admin",
            IamError::Email(_) => "email delivery failure",
            IamError::Repository(_) => "repository failure",
        };
        f.write_str(message)
    }
}

impl std::error::Error for IamError {}
