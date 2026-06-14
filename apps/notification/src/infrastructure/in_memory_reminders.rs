//! In-memory `ReminderRepository`. Stand-in until the Postgres adapter is wired.

use std::sync::RwLock;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    NotificationError, Page, PageRequest, Reminder, ReminderKind, ReminderRepository,
};

pub struct InMemoryReminderRepository {
    reminders: RwLock<Vec<Reminder>>,
}

impl InMemoryReminderRepository {
    pub fn new() -> Self {
        Self {
            reminders: RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryReminderRepository {
    fn default() -> Self {
        Self::new()
    }
}

fn poisoned() -> NotificationError {
    NotificationError::Repository("reminder store lock poisoned".to_owned())
}

#[async_trait]
impl ReminderRepository for InMemoryReminderRepository {
    async fn insert(&self, reminder: Reminder) -> Result<(), NotificationError> {
        let mut guard = self.reminders.write().map_err(|_| poisoned())?;
        guard.push(reminder);
        Ok(())
    }

    async fn exists(&self, loan_id: Uuid, kind: ReminderKind) -> Result<bool, NotificationError> {
        let found = {
            let guard = self.reminders.read().map_err(|_| poisoned())?;
            guard
                .iter()
                .any(|reminder| reminder.loan_id == loan_id && reminder.kind == kind)
        };
        Ok(found)
    }

    async fn list_by_user(
        &self,
        user_id: Uuid,
        request: PageRequest,
    ) -> Result<Page<Reminder>, NotificationError> {
        let mine: Vec<Reminder> = {
            let guard = self.reminders.read().map_err(|_| poisoned())?;
            guard
                .iter()
                .filter(|reminder| reminder.user_id == user_id)
                .cloned()
                .collect()
        };

        let total = mine.len() as u64;
        let offset = request.offset() as usize;
        let items = mine
            .into_iter()
            .skip(offset)
            .take(request.page_size() as usize)
            .collect();

        Ok(Page {
            items,
            page: request.page(),
            page_size: request.page_size(),
            total,
        })
    }
}
