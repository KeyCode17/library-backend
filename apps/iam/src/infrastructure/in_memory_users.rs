//! In-memory `UserRepository` for the contexts' DB-free unit tests.

use std::sync::RwLock;

use uuid::Uuid;

use async_trait::async_trait;

use crate::domain::{AdminGuard, IamError, Page, PageRequest, Role, User, UserRepository};

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

    async fn delete_guarding_last_admin(&self, id: Uuid) -> Result<AdminGuard<()>, IamError> {
        // The single write lock makes count-and-mutate atomic in memory.
        let mut guard = self.users.write().map_err(|_| poisoned())?;
        let Some(pos) = guard.iter().position(|user| user.id == id) else {
            return Ok(AdminGuard::NotFound);
        };
        if removes_last_admin(&guard, &guard[pos]) {
            return Ok(AdminGuard::LastAdmin);
        }
        guard.remove(pos);
        Ok(AdminGuard::Done(()))
    }

    async fn deactivate_guarding_last_admin(&self, id: Uuid) -> Result<AdminGuard<User>, IamError> {
        let mut guard = self.users.write().map_err(|_| poisoned())?;
        let Some(pos) = guard.iter().position(|user| user.id == id) else {
            return Ok(AdminGuard::NotFound);
        };
        if removes_last_admin(&guard, &guard[pos]) {
            return Ok(AdminGuard::LastAdmin);
        }
        guard[pos].active = false;
        Ok(AdminGuard::Done(guard[pos].clone()))
    }

    async fn set_role_guarding_last_admin(
        &self,
        id: Uuid,
        role: Role,
    ) -> Result<AdminGuard<User>, IamError> {
        let mut guard = self.users.write().map_err(|_| poisoned())?;
        let Some(pos) = guard.iter().position(|user| user.id == id) else {
            return Ok(AdminGuard::NotFound);
        };
        let demotes_admin = role != Role::Admin;
        if demotes_admin && removes_last_admin(&guard, &guard[pos]) {
            return Ok(AdminGuard::LastAdmin);
        }
        guard[pos].role = role;
        Ok(AdminGuard::Done(guard[pos].clone()))
    }
}

/// Whether removing `target`'s active-admin status leaves zero active admins.
fn removes_last_admin(all: &[User], target: &User) -> bool {
    let active_admins = all
        .iter()
        .filter(|user| user.role == Role::Admin && user.active)
        .count();
    target.role == Role::Admin && target.active && active_admins <= 1
}
