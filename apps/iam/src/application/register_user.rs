use std::sync::Arc;

use chrono::Duration;
use uuid::Uuid;

use crate::domain::{
    Clock, EmailSender, EmailToken, EmailTokenKind, EmailTokenRepository, IamError, PasswordHasher,
    Role, TokenGenerator, User, UserRepository,
};

use super::validation::{normalize_email, validate_email, validate_password};

/// Verification links live for 24h.
pub const VERIFY_TTL_HOURS: i64 = 24;

/// Use case: public self-registration. Creates an unverified `member` and sends a
/// verification email (best-effort — a send failure never fails the registration,
/// and login is not gated on verification).
pub struct RegisterUser {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    clock: Arc<dyn Clock>,
    email_tokens: Arc<dyn EmailTokenRepository>,
    token_generator: Arc<dyn TokenGenerator>,
    email_sender: Arc<dyn EmailSender>,
    base_url: String,
}

impl RegisterUser {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        users: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        clock: Arc<dyn Clock>,
        email_tokens: Arc<dyn EmailTokenRepository>,
        token_generator: Arc<dyn TokenGenerator>,
        email_sender: Arc<dyn EmailSender>,
        base_url: String,
    ) -> Self {
        Self {
            users,
            hasher,
            clock,
            email_tokens,
            token_generator,
            email_sender,
            base_url,
        }
    }

    pub async fn execute(&self, email: &str, password: &str) -> Result<User, IamError> {
        let email = normalize_email(email);
        validate_email(&email)?;
        validate_password(password)?;

        let password_hash = self.hasher.hash(password)?;
        let now = self.clock.now();
        let user = User::new(Uuid::new_v4(), email, password_hash, Role::Member, now);
        self.users.insert(user.clone()).await?;

        issue_and_send_verification(
            self.email_tokens.as_ref(),
            self.token_generator.as_ref(),
            self.email_sender.as_ref(),
            &self.base_url,
            &user,
            now,
        )
        .await;

        Ok(user)
    }
}

/// Mint a verification token and email its link. Best-effort: failures are logged,
/// not propagated (so registration never fails on email trouble).
pub(crate) async fn issue_and_send_verification(
    email_tokens: &dyn EmailTokenRepository,
    token_generator: &dyn TokenGenerator,
    email_sender: &dyn EmailSender,
    base_url: &str,
    user: &User,
    now: chrono::DateTime<chrono::Utc>,
) {
    let (raw, hash) = token_generator.generate();
    let token = EmailToken {
        id: Uuid::new_v4(),
        user_id: user.id,
        kind: EmailTokenKind::VerifyEmail,
        token_hash: hash,
        expires_at: now + Duration::hours(VERIFY_TTL_HOURS),
        consumed_at: None,
        created_at: now,
    };
    if let Err(error) = email_tokens.insert(token).await {
        eprintln!("WARN [iam]: could not store verification token: {error}");
        return;
    }
    let link = format!("{base_url}/verify-email?token={raw}");
    if let Err(error) = email_sender.send_verification(&user.email, &link).await {
        eprintln!("WARN [iam]: could not send verification email: {error}");
    }
}
