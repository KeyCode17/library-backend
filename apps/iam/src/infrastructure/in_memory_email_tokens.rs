//! In-memory `EmailTokenRepository` for DB-free unit tests.

use std::sync::RwLock;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use async_trait::async_trait;

use crate::domain::{EmailToken, EmailTokenRepository, IamError};

pub struct InMemoryEmailTokenRepository {
    tokens: RwLock<Vec<EmailToken>>,
}

impl InMemoryEmailTokenRepository {
    pub fn new() -> Self {
        Self {
            tokens: RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryEmailTokenRepository {
    fn default() -> Self {
        Self::new()
    }
}

fn poisoned() -> IamError {
    IamError::Repository("email token store lock poisoned".to_owned())
}

#[async_trait]
impl EmailTokenRepository for InMemoryEmailTokenRepository {
    async fn insert(&self, token: EmailToken) -> Result<(), IamError> {
        self.tokens.write().map_err(|_| poisoned())?.push(token);
        Ok(())
    }

    async fn find_by_hash(&self, token_hash: &str) -> Result<Option<EmailToken>, IamError> {
        let found = {
            let guard = self.tokens.read().map_err(|_| poisoned())?;
            guard
                .iter()
                .find(|token| token.token_hash == token_hash)
                .cloned()
        };
        Ok(found)
    }

    async fn consume(&self, id: Uuid, at: DateTime<Utc>) -> Result<bool, IamError> {
        let mut guard = self.tokens.write().map_err(|_| poisoned())?;
        match guard.iter_mut().find(|token| token.id == id) {
            Some(token) if token.consumed_at.is_none() => {
                token.consumed_at = Some(at);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
