use std::sync::Arc;

use uuid::Uuid;

use crate::domain::{AdminGuard, AuthPrincipal, IamError, Permission, Role, User, UserRepository};

/// Use case: an admin assigns a role to a user.
///
/// Authorization lives here (server-side authority). Self-demotion is refused up
/// front; the last-admin invariant is enforced transactionally in the repository
/// so it holds under concurrency.
pub struct AssignRole {
    users: Arc<dyn UserRepository>,
}

impl AssignRole {
    pub fn new(users: Arc<dyn UserRepository>) -> Self {
        Self { users }
    }

    pub async fn execute(
        &self,
        actor: &AuthPrincipal,
        target_id: Uuid,
        new_role: Role,
    ) -> Result<User, IamError> {
        if !actor.role.grants(Permission::ManageUsers) {
            return Err(IamError::Forbidden);
        }
        // An admin cannot demote themselves out of admin.
        if actor.role == Role::Admin && target_id == actor.user_id && new_role != Role::Admin {
            return Err(IamError::LastAdmin);
        }

        match self
            .users
            .set_role_guarding_last_admin(target_id, new_role)
            .await?
        {
            AdminGuard::Done(user) => Ok(user),
            AdminGuard::LastAdmin => Err(IamError::LastAdmin),
            AdminGuard::NotFound => Err(IamError::UserNotFound),
        }
    }
}
