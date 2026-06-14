//! In-memory `UserRepository`. Stand-in until the Postgres/SeaORM adapter is
//! wired (the users table schema lives in the `migration` crate).

use std::sync::RwLock;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{IamError, Role, User, UserRepository};

pub struct InMemoryUserRepository {
    users: RwLock<Vec<User>>,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self {
            users: RwLock::new(Vec::new()),
        }
    }

    /// Build a store pre-populated with `initial` users (e.g. a seeded admin).
    pub fn seeded_with(initial: Vec<User>) -> Self {
        Self {
            users: RwLock::new(initial),
        }
    }
}

impl Default for InMemoryUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

fn poisoned() -> IamError {
    IamError::Repository("user store lock poisoned".to_owned())
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, IamError> {
        let found = {
            let guard = self.users.read().map_err(|_| poisoned())?;
            guard.iter().find(|user| user.email == email).cloned()
        };
        Ok(found)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, IamError> {
        let found = {
            let guard = self.users.read().map_err(|_| poisoned())?;
            guard.iter().find(|user| user.id == id).cloned()
        };
        Ok(found)
    }

    async fn insert(&self, user: User) -> Result<(), IamError> {
        let mut guard = self.users.write().map_err(|_| poisoned())?;
        if guard.iter().any(|existing| existing.email == user.email) {
            return Err(IamError::EmailAlreadyExists);
        }
        guard.push(user);
        Ok(())
    }

    async fn set_role(&self, id: Uuid, role: Role) -> Result<Option<User>, IamError> {
        let mut guard = self.users.write().map_err(|_| poisoned())?;
        match guard.iter_mut().find(|user| user.id == id) {
            Some(user) => {
                user.role = role;
                Ok(Some(user.clone()))
            }
            None => Ok(None),
        }
    }
}
