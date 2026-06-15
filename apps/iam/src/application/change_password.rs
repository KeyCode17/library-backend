use std::sync::Arc;

use crate::domain::{AuthPrincipal, IamError, PasswordHasher, UserRepository};

use super::validation::validate_password;

/// Use case: a user changes their own password (verifying the current one).
pub struct ChangePassword {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
}

impl ChangePassword {
    pub fn new(users: Arc<dyn UserRepository>, hasher: Arc<dyn PasswordHasher>) -> Self {
        Self { users, hasher }
    }

    pub async fn execute(
        &self,
        actor: &AuthPrincipal,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), IamError> {
        let user = self
            .users
            .find_by_id(actor.user_id)
            .await?
            .ok_or(IamError::Unauthorized)?;

        if !self.hasher.verify(current_password, &user.password_hash)? {
            return Err(IamError::InvalidCredentials);
        }
        validate_password(new_password)?;

        let new_hash = self.hasher.hash(new_password)?;
        self.users
            .set_password_hash(actor.user_id, &new_hash)
            .await?
            .ok_or(IamError::UserNotFound)?;
        Ok(())
    }
}
