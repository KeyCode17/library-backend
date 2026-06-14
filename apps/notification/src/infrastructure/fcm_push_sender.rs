//! The real FCM (HTTP v1) push adapter — credential-gated.
//!
//! When `FCM_*` is unset it logs and no-ops (a deployment concern, like the JWT
//! secret); when configured it POSTs to the FCM v1 `messages:send` endpoint with
//! the configured bearer token.

use async_trait::async_trait;

use crate::domain::{NotificationError, PushNotification, PushSender};

use super::config::FcmConfig;

pub struct FcmPushSender {
    config: Option<FcmConfig>,
    client: reqwest::Client,
}

impl FcmPushSender {
    /// Build from the environment. Unconfigured senders no-op gracefully.
    pub fn from_env() -> Self {
        Self {
            config: FcmConfig::from_env(),
            client: reqwest::Client::new(),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.config.is_some()
    }
}

impl Default for FcmPushSender {
    fn default() -> Self {
        Self::from_env()
    }
}

#[async_trait]
impl PushSender for FcmPushSender {
    async fn send(
        &self,
        device_token: &str,
        notification: &PushNotification,
    ) -> Result<(), NotificationError> {
        let Some(config) = &self.config else {
            eprintln!(
                "INFO [notification]: FCM not configured; skipping push '{}' (set FCM_PROJECT_ID / FCM_ACCESS_TOKEN to enable).",
                notification.title
            );
            return Ok(());
        };

        let url = format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            config.project_id
        );
        let payload = serde_json::json!({
            "message": {
                "token": device_token,
                "notification": {
                    "title": notification.title,
                    "body": notification.body,
                }
            }
        });

        let response = self
            .client
            .post(url)
            .bearer_auth(&config.access_token)
            .json(&payload)
            .send()
            .await
            .map_err(|error| NotificationError::Push(error.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(NotificationError::Push(format!(
                "FCM responded {}",
                response.status()
            )))
        }
    }
}
