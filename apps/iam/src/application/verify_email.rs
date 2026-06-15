use std::sync::Arc;

use crate::domain::{
    Clock, EmailTokenKind, EmailTokenRepository, IamError, TokenGenerator, UserRepository,
};

/// Use case: consume a verification token and mark the user verified.
pub struct VerifyEmail {
    users: Arc<dyn UserRepository>,
    email_tokens: Arc<dyn EmailTokenRepository>,
    token_generator: Arc<dyn TokenGenerator>,
    clock: Arc<dyn Clock>,
}

impl VerifyEmail {
    pub fn new(
        users: Arc<dyn UserRepository>,
        email_tokens: Arc<dyn EmailTokenRepository>,
        token_generator: Arc<dyn TokenGenerator>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            users,
            email_tokens,
            token_generator,
            clock,
        }
    }

    pub async fn execute(&self, raw_token: &str) -> Result<(), IamError> {
        let hash = self.token_generator.hash(raw_token);
        let token = self
            .email_tokens
            .find_by_hash(&hash)
            .await?
            .ok_or(IamError::InvalidToken)?;

        if token.kind != EmailTokenKind::VerifyEmail {
            return Err(IamError::InvalidToken);
        }
        let now = self.clock.now();
        if token.consumed_at.is_some() {
            return Err(IamError::TokenConsumed);
        }
        if token.expires_at <= now {
            return Err(IamError::TokenExpired);
        }
        // Single-use: consume atomically (loses a race => already consumed).
        if !self.email_tokens.consume(token.id, now).await? {
            return Err(IamError::TokenConsumed);
        }

        self.users
            .set_verified(token.user_id, true)
            .await?
            .ok_or(IamError::UserNotFound)?;
        Ok(())
    }
}
