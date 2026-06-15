use std::sync::Arc;

use uuid::Uuid;

use crate::domain::{AdminGuard, AuthPrincipal, IamError, Permission, User, UserRepository};

use super::validation::{normalize_email, validate_email};

/// Use case: admin updates a user's email and/or active flag. Deactivating
/// yourself is refused up front; deactivating the last active admin is refused
/// transactionally in the repository.
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
        if active == Some(false) && target_id == actor.user_id {
            return Err(IamError::LastAdmin); // can't deactivate yourself
        }

        let mut current: Option<User> = None;

        if let Some(email) = email {
            let email = normalize_email(&email);
            validate_email(&email)?;
            current = Some(
                self.users
                    .set_email(target_id, &email)
                    .await?
                    .ok_or(IamError::UserNotFound)?,
            );
        }

        match active {
            Some(false) => {
                current = Some(
                    match self.users.deactivate_guarding_last_admin(target_id).await? {
                        AdminGuard::Done(user) => user,
                        AdminGuard::LastAdmin => return Err(IamError::LastAdmin),
                        AdminGuard::NotFound => return Err(IamError::UserNotFound),
                    },
                );
            }
            Some(true) => {
                current = Some(
                    self.users
                        .set_active(target_id, true)
                        .await?
                        .ok_or(IamError::UserNotFound)?,
                );
            }
            None => {}
        }

        match current {
            Some(user) => Ok(user),
            None => self
                .users
                .find_by_id(target_id)
                .await?
                .ok_or(IamError::UserNotFound),
        }
    }
}
