//! A push sender that records what it "sent" — for dev logging and tests.

use std::sync::Mutex;

use async_trait::async_trait;

use crate::domain::{NotificationError, PushNotification, PushSender};

pub struct FakePushSender {
    sent: Mutex<Vec<(String, PushNotification)>>,
}

impl FakePushSender {
    pub fn new() -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
        }
    }

    /// Every (token, notification) pair pushed so far.
    pub fn sent(&self) -> Vec<(String, PushNotification)> {
        self.sent.lock().expect("fake push lock").clone()
    }
}

impl Default for FakePushSender {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PushSender for FakePushSender {
    async fn send(
        &self,
        device_token: &str,
        notification: &PushNotification,
    ) -> Result<(), NotificationError> {
        self.sent
            .lock()
            .expect("fake push lock")
            .push((device_token.to_owned(), notification.clone()));
        Ok(())
    }
}
