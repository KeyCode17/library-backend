use std::sync::Arc;

use uuid::Uuid;

use crate::domain::{
    AuthPrincipal, Clock, IamError, PasswordHasher, Permission, Role, User, UserRepository,
};

use super::validation::{normalize_email, validate_email, validate_password};

/// Use case: admin creates a user with an explicit role.
pub struct CreateUser {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    clock: Arc<dyn Clock>,
}

impl CreateUser {
    pub fn new(
        users: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            users,
            hasher,
            clock,
        }
    }

    pub async fn execute(
        &self,
        actor: &AuthPrincipal,
        email: &str,
        password: &str,
        role: Role,
    ) -> Result<User, IamError> {
        if !actor.role.grants(Permission::ManageUsers) {
            return Err(IamError::Forbidden);
        }
        let email = normalize_email(email);
        validate_email(&email)?;
        validate_password(password)?;

        let password_hash = self.hasher.hash(password)?;
        let user = User::new(Uuid::new_v4(), email, password_hash, role, self.clock.now());
        self.users.insert(user.clone()).await?;
        Ok(user)
    }
}
