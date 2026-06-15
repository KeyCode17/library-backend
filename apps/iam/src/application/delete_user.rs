use std::sync::Arc;

use uuid::Uuid;

use crate::domain::{AdminGuard, AuthPrincipal, IamError, Permission, UserRepository};

/// Use case: admin deletes a user. Deleting yourself is refused up front; the
/// last-admin invariant is enforced transactionally in the repository.
pub struct DeleteUser {
    users: Arc<dyn UserRepository>,
}

impl DeleteUser {
    pub fn new(users: Arc<dyn UserRepository>) -> Self {
        Self { users }
    }

    pub async fn execute(&self, actor: &AuthPrincipal, target_id: Uuid) -> Result<(), IamError> {
        if !actor.role.grants(Permission::ManageUsers) {
            return Err(IamError::Forbidden);
        }
        if target_id == actor.user_id {
            return Err(IamError::LastAdmin); // use DELETE /auth/me to delete yourself
        }

        match self.users.delete_guarding_last_admin(target_id).await? {
            AdminGuard::Done(()) => Ok(()),
            AdminGuard::LastAdmin => Err(IamError::LastAdmin),
            AdminGuard::NotFound => Err(IamError::UserNotFound),
        }
    }
}
