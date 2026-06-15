use std::sync::Arc;

use uuid::Uuid;

use crate::domain::{AuthPrincipal, IamError, Permission, Role, User, UserRepository};

use super::validation::{normalize_email, validate_email};

/// Use case: admin updates a user's email and/or active flag. Deactivating
/// yourself or the last admin is refused (lockout safety).
pub struct UpdateUser {
    users: Arc<dyn UserRepository>,
}

impl UpdateUser {
    pub fn new(users: Arc<dyn UserRepository>) -> Self {
        Self { users }
    }

    pub async fn execute(
        &self,
        actor: &AuthPrincipal,
        target_id: Uuid,
        email: Option<String>,
        active: Option<bool>,
    ) -> Result<User, IamError> {
        if !actor.role.grants(Permission::ManageUsers) {
            return Err(IamError::Forbidden);
        }

        let target = self
            .users
            .find_by_id(target_id)
            .await?
            .ok_or(IamError::UserNotFound)?;

        if active == Some(false) {
            if target.id == actor.user_id {
                return Err(IamError::LastAdmin);
            }
            if target.role == Role::Admin && self.users.count_active_admins().await? <= 1 {
                return Err(IamError::LastAdmin);
            }
        }

        let mut updated = target;
        if let Some(email) = email {
            let email = normalize_email(&email);
            validate_email(&email)?;
            updated = self
                .users
                .set_email(target_id, &email)
                .await?
                .ok_or(IamError::UserNotFound)?;
        }
        if let Some(active) = active {
            updated = self
                .users
                .set_active(target_id, active)
                .await?
                .ok_or(IamError::UserNotFound)?;
        }
        Ok(updated)
    }
}
