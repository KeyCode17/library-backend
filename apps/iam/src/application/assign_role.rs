use std::sync::Arc;

use uuid::Uuid;

use crate::domain::{AuthPrincipal, IamError, Permission, Role, User, UserRepository};

/// Use case: an admin assigns a role to a user.
///
/// Authorization lives here, in the application (the server-side authority) — not
/// only at the HTTP edge. Demoting the last admin, or an admin demoting
/// themselves, is refused to prevent lockout.
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

        let target = self
            .users
            .find_by_id(target_id)
            .await?
            .ok_or(IamError::UserNotFound)?;

        if target.role == Role::Admin && new_role != Role::Admin {
            // Demoting an admin: never the caller themselves, never the last one.
            if target.id == actor.user_id {
                return Err(IamError::LastAdmin);
            }
            if self.users.count_active_admins().await? <= 1 {
                return Err(IamError::LastAdmin);
            }
        }

        self.users
            .set_role(target_id, new_role)
            .await?
            .ok_or(IamError::UserNotFound)
    }
}
