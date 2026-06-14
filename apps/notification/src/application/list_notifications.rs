use std::sync::Arc;

use uuid::Uuid;

use crate::domain::{NotificationError, Page, PageRequest, Reminder, ReminderRepository};

/// Use case: a user's reminder history.
pub struct ListNotifications {
    reminders: Arc<dyn ReminderRepository>,
}

impl ListNotifications {
    pub fn new(reminders: Arc<dyn ReminderRepository>) -> Self {
        Self { reminders }
    }

    pub async fn execute(
        &self,
        user_id: Uuid,
        request: PageRequest,
    ) -> Result<Page<Reminder>, NotificationError> {
        self.reminders.list_by_user(user_id, request).await
    }
}
