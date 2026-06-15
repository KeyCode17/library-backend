use std::sync::Arc;

use crate::domain::{AuthPrincipal, IamError, User, UserRepository};

use super::validation::{normalize_email, validate_email};

/// Use case: a user updates their own email.
pub struct UpdateMe {
    users: Arc<dyn UserRepository>,
}

impl UpdateMe {
    pub fn new(users: Arc<dyn UserRepository>) -> Self {
        Self { users }
    }

    pub async fn execute(&self, actor: &AuthPrincipal, email: &str) -> Result<User, IamError> {
        let email = normalize_email(email);
        validate_email(&email)?;
        self.users
            .set_email(actor.user_id, &email)
            .await?
            .ok_or(IamError::Unauthorized)
    }
}
