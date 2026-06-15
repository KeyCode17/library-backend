use std::sync::Arc;

use uuid::Uuid;

use crate::domain::{AuthPrincipal, IamError, Permission, Role, UserRepository};

/// Use case: admin deletes a user. Deleting yourself or the last admin is refused.
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

        let target = self
            .users
            .find_by_id(target_id)
            .await?
            .ok_or(IamError::UserNotFound)?;

        if target.id == actor.user_id {
            return Err(IamError::LastAdmin);
        }
        if target.role == Role::Admin && self.users.count_active_admins().await? <= 1 {
            return Err(IamError::LastAdmin);
        }

        if !self.users.delete(target_id).await? {
            return Err(IamError::UserNotFound);
        }
        Ok(())
    }
}
