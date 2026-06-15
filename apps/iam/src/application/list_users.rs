use std::sync::Arc;

use crate::domain::{AuthPrincipal, IamError, Page, PageRequest, Permission, User, UserRepository};

/// Use case: admin lists users (paginated).
pub struct ListUsers {
    users: Arc<dyn UserRepository>,
}

impl ListUsers {
    pub fn new(users: Arc<dyn UserRepository>) -> Self {
        Self { users }
    }

    pub async fn execute(
        &self,
        actor: &AuthPrincipal,
        request: PageRequest,
    ) -> Result<Page<User>, IamError> {
        if !actor.role.grants(Permission::ManageUsers) {
            return Err(IamError::Forbidden);
        }
        self.users.list(request).await
    }
}
