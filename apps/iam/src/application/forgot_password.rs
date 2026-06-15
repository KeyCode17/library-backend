use std::sync::Arc;

use chrono::Duration;
use uuid::Uuid;

use crate::domain::{
    Clock, EmailSender, EmailToken, EmailTokenKind, EmailTokenRepository, IamError, TokenGenerator,
    UserRepository,
};

use super::validation::normalize_email;

/// Password-reset links live for 1h.
pub const RESET_TTL_HOURS: i64 = 1;

/// Use case: start password reset. Always succeeds from the caller's view (the
/// handler returns 202 regardless) — no user enumeration. If the email exists, a
/// reset token is issued and emailed (best-effort).
pub struct ForgotPassword {
    users: Arc<dyn UserRepository>,
    email_tokens: Arc<dyn EmailTokenRepository>,
    token_generator: Arc<dyn TokenGenerator>,
    email_sender: Arc<dyn EmailSender>,
    clock: Arc<dyn Clock>,
    base_url: String,
}

impl ForgotPassword {
    pub fn new(
        users: Arc<dyn UserRepository>,
        email_tokens: Arc<dyn EmailTokenRepository>,
        token_generator: Arc<dyn TokenGenerator>,
        email_sender: Arc<dyn EmailSender>,
        clock: Arc<dyn Clock>,
        base_url: String,
    ) -> Self {
        Self {
            users,
            email_tokens,
            token_generator,
            email_sender,
            clock,
            base_url,
        }
    }

    pub async fn execute(&self, email: &str) -> Result<(), IamError> {
        let email = normalize_email(email);
        let Some(user) = self.users.find_by_email(&email).await? else {
            return Ok(()); // unknown email — say nothing
        };

        let now = self.clock.now();
        let (raw, hash) = self.token_generator.generate();
        let token = EmailToken {
            id: Uuid::new_v4(),
            user_id: user.id,
            kind: EmailTokenKind::PasswordReset,
            token_hash: hash,
            expires_at: now + Duration::hours(RESET_TTL_HOURS),
            consumed_at: None,
            created_at: now,
        };
        if self.email_tokens.insert(token).await.is_ok() {
            let link = format!("{}/reset-password?token={}", self.base_url, raw);
            if let Err(error) = self
                .email_sender
                .send_password_reset(&user.email, &link)
                .await
            {
                eprintln!("WARN [iam]: could not send reset email: {error}");
            }
        }
        Ok(())
    }
}
