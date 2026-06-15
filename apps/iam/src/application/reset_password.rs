use std::sync::Arc;

use crate::domain::{
    Clock, EmailTokenKind, EmailTokenRepository, IamError, PasswordHasher, TokenGenerator,
    UserRepository,
};

use super::validation::validate_password;

/// Use case: consume a reset token and set a new password.
pub struct ResetPassword {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    email_tokens: Arc<dyn EmailTokenRepository>,
    token_generator: Arc<dyn TokenGenerator>,
    clock: Arc<dyn Clock>,
}

impl ResetPassword {
    pub fn new(
        users: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        email_tokens: Arc<dyn EmailTokenRepository>,
        token_generator: Arc<dyn TokenGenerator>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            users,
            hasher,
            email_tokens,
            token_generator,
            clock,
        }
    }

    pub async fn execute(&self, raw_token: &str, new_password: &str) -> Result<(), IamError> {
        validate_password(new_password)?;

        let hash = self.token_generator.hash(raw_token);
        let token = self
            .email_tokens
            .find_by_hash(&hash)
            .await?
            .ok_or(IamError::InvalidToken)?;

        if token.kind != EmailTokenKind::PasswordReset {
            return Err(IamError::InvalidToken);
        }
        let now = self.clock.now();
        if token.consumed_at.is_some() {
            return Err(IamError::TokenConsumed);
        }
        if token.expires_at <= now {
            return Err(IamError::TokenExpired);
        }
        if !self.email_tokens.consume(token.id, now).await? {
            return Err(IamError::TokenConsumed);
        }

        let new_hash = self.hasher.hash(new_password)?;
        self.users
            .set_password_hash(token.user_id, &new_hash)
            .await?
            .ok_or(IamError::UserNotFound)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Clock, EmailToken, EmailTokenKind, Role, TokenGenerator, User};
    use crate::infrastructure::{
        Argon2PasswordHasher, InMemoryEmailTokenRepository, InMemoryUserRepository,
        RandomTokenGenerator,
    };
    use chrono::{DateTime, Duration, Utc};
    use uuid::Uuid;

    struct FixedClock(DateTime<Utc>);
    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).expect("timestamp")
    }

    struct Fixture {
        reset: ResetPassword,
        tokens: Arc<InMemoryEmailTokenRepository>,
        generator: Arc<RandomTokenGenerator>,
        user_id: Uuid,
    }

    fn fixture() -> Fixture {
        let user_id = Uuid::new_v4();
        let user = User::new(
            user_id,
            "u@example.com".into(),
            "$argon2id$placeholder".into(),
            Role::Member,
            now(),
        );
        let users = Arc::new(InMemoryUserRepository::seeded_with(vec![user]));
        let tokens = Arc::new(InMemoryEmailTokenRepository::new());
        let generator = Arc::new(RandomTokenGenerator::new());
        let reset = ResetPassword::new(
            users,
            Arc::new(Argon2PasswordHasher::new()),
            tokens.clone(),
            generator.clone(),
            Arc::new(FixedClock(now())),
        );
        Fixture {
            reset,
            tokens,
            generator,
            user_id,
        }
    }

    async fn store_token(f: &Fixture, kind: EmailTokenKind, expires_at: DateTime<Utc>) -> String {
        let (raw, hash) = f.generator.generate();
        f.tokens
            .insert(EmailToken {
                id: Uuid::new_v4(),
                user_id: f.user_id,
                kind,
                token_hash: hash,
                expires_at,
                consumed_at: None,
                created_at: now(),
            })
            .await
            .expect("store token");
        raw
    }

    #[tokio::test]
    async fn valid_reset_token_sets_the_password() {
        let f = fixture();
        let raw = store_token(
            &f,
            EmailTokenKind::PasswordReset,
            now() + Duration::hours(1),
        )
        .await;
        assert!(f.reset.execute(&raw, "newpassword1").await.is_ok());
    }

    #[tokio::test]
    async fn expired_reset_token_is_rejected() {
        let f = fixture();
        let raw = store_token(
            &f,
            EmailTokenKind::PasswordReset,
            now() - Duration::minutes(1),
        )
        .await;
        assert!(matches!(
            f.reset.execute(&raw, "newpassword1").await,
            Err(IamError::TokenExpired)
        ));
    }

    #[tokio::test]
    async fn a_verification_token_cannot_reset_a_password() {
        let f = fixture();
        let raw = store_token(&f, EmailTokenKind::VerifyEmail, now() + Duration::hours(1)).await;
        assert!(matches!(
            f.reset.execute(&raw, "newpassword1").await,
            Err(IamError::InvalidToken)
        ));
    }

    #[tokio::test]
    async fn unknown_token_is_rejected() {
        let f = fixture();
        assert!(matches!(
            f.reset.execute("deadbeef", "newpassword1").await,
            Err(IamError::InvalidToken)
        ));
    }
}
