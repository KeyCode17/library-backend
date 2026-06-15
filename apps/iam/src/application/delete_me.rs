use std::sync::Arc;

use crate::domain::{AuthPrincipal, IamError, Role, UserRepository};

/// Use case: a user deletes their own account. Blocked if they are the last admin.
pub struct DeleteMe {
    users: Arc<dyn UserRepository>,
}

impl DeleteMe {
    pub fn new(users: Arc<dyn UserRepository>) -> Self {
        Self { users }
    }

    pub async fn execute(&self, actor: &AuthPrincipal) -> Result<(), IamError> {
        let user = self
            .users
            .find_by_id(actor.user_id)
            .await?
            .ok_or(IamError::Unauthorized)?;

        if user.role == Role::Admin && self.users.count_active_admins().await? <= 1 {
            return Err(IamError::LastAdmin);
        }

        if !self.users.delete(actor.user_id).await? {
            return Err(IamError::Unauthorized);
        }
        Ok(())
    }
}
