use std::sync::Arc;

use crate::domain::{
    AuthPrincipal, IamError, IssuedToken, PasswordHasher, TokenService, UserRepository,
};

/// Use case: exchange credentials for a signed token.
///
/// Unknown email and wrong password both return `InvalidCredentials`, and the
/// unknown-email path still performs a hash to keep timing similar — neither
/// outcome lets a caller enumerate which emails exist.
pub struct LoginUser {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    tokens: Arc<dyn TokenService>,
}

impl LoginUser {
    pub fn new(
        users: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        tokens: Arc<dyn TokenService>,
    ) -> Self {
        Self {
            users,
            hasher,
            tokens,
        }
    }

    pub async fn execute(&self, email: &str, password: &str) -> Result<IssuedToken, IamError> {
        let email = email.trim().to_lowercase();

        let Some(user) = self.users.find_by_email(&email).await? else {
            // Spend comparable time hashing so a missing user is indistinguishable
            // from a wrong password by timing.
            let _ = self.hasher.hash(password);
            return Err(IamError::InvalidCredentials);
        };

        if self.hasher.verify(password, &user.password_hash)? {
            let principal = AuthPrincipal {
                user_id: user.id,
                role: user.role,
            };
            self.tokens.issue(&principal)
        } else {
            Err(IamError::InvalidCredentials)
        }
    }
}
