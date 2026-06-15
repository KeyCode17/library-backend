//! Shared input validation for credentials.

use crate::domain::IamError;

/// Minimum password length enforced everywhere a password is set.
pub const MIN_PASSWORD_LEN: usize = 8;

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub fn validate_email(email: &str) -> Result<(), IamError> {
    let valid =
        email.len() >= 3 && email.contains('@') && !email.starts_with('@') && !email.ends_with('@');
    if valid {
        Ok(())
    } else {
        Err(IamError::InvalidEmail)
    }
}

pub fn validate_password(password: &str) -> Result<(), IamError> {
    if password.len() >= MIN_PASSWORD_LEN {
        Ok(())
    } else {
        Err(IamError::WeakPassword)
    }
}
