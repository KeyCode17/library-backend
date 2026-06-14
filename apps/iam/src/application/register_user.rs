use std::sync::Arc;

use uuid::Uuid;

use crate::domain::{IamError, PasswordHasher, Role, User, UserRepository};

/// Minimum password length enforced at registration.
pub const MIN_PASSWORD_LEN: usize = 8;

/// Use case: public self-registration. Always creates a `member`; elevated roles
/// are granted only by an admin via `AssignRole`.
pub struct RegisterUser {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
}

impl RegisterUser {
    pub fn new(users: Arc<dyn UserRepository>, hasher: Arc<dyn PasswordHasher>) -> Self {
        Self { users, hasher }
    }

    pub async fn execute(&self, email: &str, password: &str) -> Result<User, IamError> {
        let email = normalize_email(email);
        validate_email(&email)?;
        validate_password(password)?;

        let password_hash = self.hasher.hash(password)?;
        let user = User::new(Uuid::new_v4(), email, password_hash, Role::Member);
        self.users.insert(user.clone()).await?;
        Ok(user)
    }
}

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

fn validate_email(email: &str) -> Result<(), IamError> {
    let valid =
        email.len() >= 3 && email.contains('@') && !email.starts_with('@') && !email.ends_with('@');
    if valid {
        Ok(())
    } else {
        Err(IamError::InvalidEmail)
    }
}

fn validate_password(password: &str) -> Result<(), IamError> {
    if password.len() >= MIN_PASSWORD_LEN {
        Ok(())
    } else {
        Err(IamError::WeakPassword)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::argon2_hasher::Argon2PasswordHasher;
    use crate::infrastructure::in_memory_users::InMemoryUserRepository;

    fn use_case() -> (Arc<InMemoryUserRepository>, RegisterUser) {
        let users = Arc::new(InMemoryUserRepository::new());
        let hasher = Arc::new(Argon2PasswordHasher::new());
        let register = RegisterUser::new(users.clone(), hasher);
        (users, register)
    }

    #[tokio::test]
    async fn registers_a_member_with_a_hashed_password() {
        let (_users, register) = use_case();
        let user = register
            .execute("Alice@Example.com ", "supersecret")
            .await
            .expect("registration succeeds");

        assert_eq!(user.role, Role::Member);
        assert_eq!(user.email, "alice@example.com"); // normalized
        assert_ne!(user.password_hash, "supersecret"); // never plaintext
        assert!(user.password_hash.starts_with("$argon2"));
    }

    #[tokio::test]
    async fn rejects_duplicate_email() {
        let (_users, register) = use_case();
        register
            .execute("a@b.com", "supersecret")
            .await
            .expect("first registration");
        let err = register
            .execute("a@b.com", "supersecret")
            .await
            .unwrap_err();
        assert!(matches!(err, IamError::EmailAlreadyExists));
    }

    #[tokio::test]
    async fn rejects_short_password_and_bad_email() {
        let (_users, register) = use_case();
        assert!(matches!(
            register.execute("a@b.com", "short").await.unwrap_err(),
            IamError::WeakPassword
        ));
        assert!(matches!(
            register
                .execute("not-an-email", "supersecret")
                .await
                .unwrap_err(),
            IamError::InvalidEmail
        ));
    }
}
