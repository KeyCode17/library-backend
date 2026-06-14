use std::sync::Arc;

use uuid::Uuid;

use crate::domain::{IamError, User, UserRepository};

/// Use case: resolve the full record for an authenticated principal. A valid
/// token whose user no longer exists is treated as `Unauthorized`.
pub struct GetCurrentUser {
    users: Arc<dyn UserRepository>,
}

impl GetCurrentUser {
    pub fn new(users: Arc<dyn UserRepository>) -> Self {
        Self { users }
    }

    pub async fn execute(&self, user_id: Uuid) -> Result<User, IamError> {
        self.users
            .find_by_id(user_id)
            .await?
            .ok_or(IamError::Unauthorized)
    }
}
