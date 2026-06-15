use std::sync::Arc;

use crate::domain::{AdminGuard, AuthPrincipal, IamError, UserRepository};

/// Use case: a user deletes their own account. Refused transactionally if they
/// are the last active admin.
pub struct DeleteMe {
    users: Arc<dyn UserRepository>,
}

impl DeleteMe {
    pub fn new(users: Arc<dyn UserRepository>) -> Self {
        Self { users }
    }

    pub async fn execute(&self, actor: &AuthPrincipal) -> Result<(), IamError> {
        match self.users.delete_guarding_last_admin(actor.user_id).await? {
            AdminGuard::Done(()) => Ok(()),
            AdminGuard::LastAdmin => Err(IamError::LastAdmin),
            AdminGuard::NotFound => Err(IamError::Unauthorized),
        }
    }
}
