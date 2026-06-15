//! In-memory `UserRepository` for the contexts' DB-free unit tests.

use std::sync::RwLock;

use uuid::Uuid;

use async_trait::async_trait;

use crate::domain::{IamError, Page, PageRequest, Role, User, UserRepository};

pub struct InMemoryUserRepository {
    users: RwLock<Vec<User>>,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self {
            users: RwLock::new(Vec::new()),
        }
    }

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

    async fn list(&self, request: PageRequest) -> Result<Page<User>, IamError> {
        let all: Vec<User> = {
            let guard = self.users.read().map_err(|_| poisoned())?;
            guard.clone()
        };
        let total = all.len() as u64;
        let items = all
            .into_iter()
            .skip(request.offset() as usize)
            .take(request.page_size() as usize)
            .collect();
        Ok(Page {
            items,
            page: request.page(),
            page_size: request.page_size(),
            total,
        })
    }

    async fn set_email(&self, id: Uuid, email: &str) -> Result<Option<User>, IamError> {
        let mut guard = self.users.write().map_err(|_| poisoned())?;
        if guard
            .iter()
            .any(|user| user.email == email && user.id != id)
        {
            return Err(IamError::EmailAlreadyExists);
        }
        match guard.iter_mut().find(|user| user.id == id) {
            Some(user) => {
                user.email = email.to_owned();
                Ok(Some(user.clone()))
            }
            None => Ok(None),
        }
    }

    async fn set_active(&self, id: Uuid, active: bool) -> Result<Option<User>, IamError> {
        let mut guard = self.users.write().map_err(|_| poisoned())?;
        match guard.iter_mut().find(|user| user.id == id) {
            Some(user) => {
                user.active = active;
                Ok(Some(user.clone()))
            }
            None => Ok(None),
        }
    }

    async fn set_password_hash(&self, id: Uuid, hash: &str) -> Result<Option<User>, IamError> {
        let mut guard = self.users.write().map_err(|_| poisoned())?;
        match guard.iter_mut().find(|user| user.id == id) {
            Some(user) => {
                user.password_hash = hash.to_owned();
                Ok(Some(user.clone()))
            }
            None => Ok(None),
        }
    }

    async fn set_verified(&self, id: Uuid, verified: bool) -> Result<Option<User>, IamError> {
        let mut guard = self.users.write().map_err(|_| poisoned())?;
        match guard.iter_mut().find(|user| user.id == id) {
            Some(user) => {
                user.verified = verified;
                Ok(Some(user.clone()))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, id: Uuid) -> Result<bool, IamError> {
        let mut guard = self.users.write().map_err(|_| poisoned())?;
        let before = guard.len();
        guard.retain(|user| user.id != id);
        Ok(guard.len() != before)
    }

    async fn count_active_admins(&self) -> Result<u64, IamError> {
        let guard = self.users.read().map_err(|_| poisoned())?;
        Ok(guard
            .iter()
            .filter(|user| user.role == Role::Admin && user.active)
            .count() as u64)
    }
}
