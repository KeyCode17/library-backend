use std::sync::Arc;

use uuid::Uuid;

use crate::domain::{AuthPrincipal, IamError, Permission, Role, User, UserRepository};

/// Use case: an admin assigns a role to a user.
///
/// Authorization lives here, in the application (the server-side authority, per
/// PRD §4) — not only at the HTTP edge — so it cannot be bypassed by a caller
/// that reaches the use case another way.
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

        self.users
            .set_role(target_id, new_role)
            .await?
            .ok_or(IamError::UserNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::User;
    use crate::infrastructure::in_memory_users::InMemoryUserRepository;

    fn member(id: Uuid) -> User {
        User::new(id, "m@b.com".into(), "$argon2id$dummy".into(), Role::Member)
    }

    #[tokio::test]
    async fn admin_can_promote_a_member() {
        let target = Uuid::new_v4();
        let users = Arc::new(InMemoryUserRepository::seeded_with(vec![member(target)]));
        let assign = AssignRole::new(users);
        let admin = AuthPrincipal {
            user_id: Uuid::new_v4(),
            role: Role::Admin,
        };

        let updated = assign
            .execute(&admin, target, Role::Librarian)
            .await
            .expect("admin may assign");
        assert_eq!(updated.role, Role::Librarian);
    }

    #[tokio::test]
    async fn non_admin_is_forbidden() {
        let target = Uuid::new_v4();
        let users = Arc::new(InMemoryUserRepository::seeded_with(vec![member(target)]));
        let assign = AssignRole::new(users);
        let librarian = AuthPrincipal {
            user_id: Uuid::new_v4(),
            role: Role::Librarian,
        };

        let err = assign
            .execute(&librarian, target, Role::Admin)
            .await
            .unwrap_err();
        assert!(matches!(err, IamError::Forbidden));
    }

    #[tokio::test]
    async fn admin_promoting_missing_user_is_not_found() {
        let users = Arc::new(InMemoryUserRepository::new());
        let assign = AssignRole::new(users);
        let admin = AuthPrincipal {
            user_id: Uuid::new_v4(),
            role: Role::Admin,
        };

        let err = assign
            .execute(&admin, Uuid::new_v4(), Role::Member)
            .await
            .unwrap_err();
        assert!(matches!(err, IamError::UserNotFound));
    }
}
