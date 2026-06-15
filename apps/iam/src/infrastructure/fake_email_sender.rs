//! An email sender that records what it "sent" — for dev logging and tests.

use std::sync::Mutex;

use async_trait::async_trait;

use crate::domain::{EmailSender, IamError};

/// A captured email.
#[derive(Debug, Clone)]
pub struct SentEmail {
    pub kind: &'static str,
    pub to: String,
    pub link: String,
}

pub struct FakeEmailSender {
    sent: Mutex<Vec<SentEmail>>,
}

impl FakeEmailSender {
    pub fn new() -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
        }
    }

    pub fn sent(&self) -> Vec<SentEmail> {
        self.sent.lock().expect("fake email lock").clone()
    }

    fn record(&self, kind: &'static str, to: &str, link: &str) {
        self.sent.lock().expect("fake email lock").push(SentEmail {
            kind,
            to: to.to_owned(),
            link: link.to_owned(),
        });
    }
}

impl Default for FakeEmailSender {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmailSender for FakeEmailSender {
    async fn send_verification(&self, email: &str, link: &str) -> Result<(), IamError> {
        self.record("verify", email, link);
        Ok(())
    }
    async fn send_password_reset(&self, email: &str, link: &str) -> Result<(), IamError> {
        self.record("reset", email, link);
        Ok(())
    }
}
