//! FCM configuration from the environment.

/// FCM v1 credentials. Absent (→ `None`) means push is a logged no-op.
///
/// `FCM_ACCESS_TOKEN` is a short-lived OAuth2 bearer; minting it from a Firebase
/// service account is a deployment concern tracked for 0.9 (like `IAM_JWT_SECRET`).
pub struct FcmConfig {
    pub project_id: String,
    pub access_token: String,
}

impl FcmConfig {
    /// Read config, returning `None` unless both `FCM_PROJECT_ID` and
    /// `FCM_ACCESS_TOKEN` are set.
    pub fn from_env() -> Option<Self> {
        Some(Self {
            project_id: env_nonempty("FCM_PROJECT_ID")?,
            access_token: env_nonempty("FCM_ACCESS_TOKEN")?,
        })
    }
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}
