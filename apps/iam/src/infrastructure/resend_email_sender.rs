//! The real Resend email adapter — credential-gated.
//!
//! When `RESEND_API_KEY` is unset it logs and no-ops (a deployment concern, like
//! the JWT secret); when configured it POSTs to the Resend API.

use async_trait::async_trait;

use crate::domain::{EmailSender, IamError};
use crate::infrastructure::config::is_production;

const RESEND_URL: &str = "https://api.resend.com/emails";
const DEFAULT_FROM: &str = "onboarding@resend.dev";

pub struct ResendEmailSender {
    api_key: Option<String>,
    from: String,
    client: reqwest::Client,
}

impl ResendEmailSender {
    pub fn from_env() -> Self {
        // Fail closed in production (like the JWT secret / DATABASE_URL): refuse to
        // start rather than silently dropping every email.
        let api_key = resolve_api_key(is_production(), env_nonempty("RESEND_API_KEY"))
            .unwrap_or_else(|message| panic!("{message}"));
        let from = env_nonempty("RESEND_FROM").unwrap_or_else(|| DEFAULT_FROM.to_owned());
        Self {
            api_key,
            from,
            client: reqwest::Client::new(),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }

    async fn send(&self, to: &str, subject: &str, html: &str) -> Result<(), IamError> {
        let Some(api_key) = &self.api_key else {
            eprintln!(
                "INFO [iam]: RESEND_API_KEY unset; skipping email '{subject}' to {to} \
                 (set RESEND_API_KEY to enable)."
            );
            return Ok(());
        };

        let payload = serde_json::json!({
            "from": self.from,
            "to": to,
            "subject": subject,
            "html": html,
        });
        let response = self
            .client
            .post(RESEND_URL)
            .bearer_auth(api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|error| IamError::Email(error.to_string()))?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(IamError::Email(format!(
                "Resend responded {}",
                response.status()
            )))
        }
    }
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// Pure prod/dev policy for the API key (testable without touching env):
/// present → use it; prod + missing → error (fail closed); dev + missing → `None`
/// (log-no-op).
fn resolve_api_key(is_production: bool, api_key: Option<String>) -> Result<Option<String>, String> {
    match (api_key, is_production) {
        (Some(key), _) => Ok(Some(key)),
        (None, true) => Err(
            "RESEND_API_KEY must be set in production (APP_ENV/RUST_ENV=production); \
             refusing to start with email disabled."
                .to_owned(),
        ),
        (None, false) => Ok(None),
    }
}

#[async_trait]
impl EmailSender for ResendEmailSender {
    async fn send_verification(&self, email: &str, link: &str) -> Result<(), IamError> {
        let html = format!("<p>Confirm your email by visiting <a href=\"{link}\">{link}</a>.</p>");
        self.send(email, "Verify your email", &html).await
    }

    async fn send_password_reset(&self, email: &str, link: &str) -> Result<(), IamError> {
        let html = format!(
            "<p>Reset your password by visiting <a href=\"{link}\">{link}</a>. \
             This link expires in 1 hour.</p>"
        );
        self.send(email, "Reset your password", &html).await
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_api_key;

    #[test]
    fn key_present_is_used_anywhere() {
        assert_eq!(
            resolve_api_key(true, Some("re_123".into())).unwrap(),
            Some("re_123".to_owned())
        );
    }

    #[test]
    fn missing_key_fails_closed_in_production() {
        assert!(resolve_api_key(true, None).is_err());
    }

    #[test]
    fn missing_key_is_a_dev_no_op() {
        assert_eq!(resolve_api_key(false, None).unwrap(), None);
    }
}
