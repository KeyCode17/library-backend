//! Environment-driven IAM configuration and admin seeding.
//!
//! No secret is hardcoded or committed. The JWT signing secret comes from
//! `IAM_JWT_SECRET`; if unset, an ephemeral random secret is generated for dev
//! (tokens do not survive a restart). The seed admin password comes from
//! `IAM_ADMIN_PASSWORD`; if unset, a random dev password is generated and logged
//! at startup — never stored in the repo.

use std::fmt::Write as _;

use rand_core::{OsRng, RngCore};
use uuid::Uuid;

use crate::domain::{IamError, PasswordHasher, Role, User};

const DEFAULT_ADMIN_EMAIL: &str = "admin@library.local";
const DEFAULT_TTL_SECS: u64 = 3600;
const DEFAULT_BASE_URL: &str = "http://localhost:8080";

pub struct IamConfig {
    pub jwt_secret: Vec<u8>,
    pub token_ttl_secs: u64,
    pub admin_email: String,
    pub admin_password: String,
    /// Public base URL used to build verification/reset links (`APP_PUBLIC_URL`).
    pub public_base_url: String,
}

impl IamConfig {
    /// Read config from the environment, applying safe dev fallbacks with loud
    /// warnings. In production (`APP_ENV`/`RUST_ENV` = `production`/`prod`) a
    /// missing `IAM_JWT_SECRET` is fatal — fail closed rather than mint tokens
    /// with a random per-boot secret.
    pub fn from_env() -> Self {
        let jwt_secret = resolve_jwt_secret(
            is_production(),
            env_nonempty("IAM_JWT_SECRET").map(String::into_bytes),
        )
        .unwrap_or_else(|message| panic!("{message}"));

        let token_ttl_secs = env_nonempty("IAM_TOKEN_TTL_SECS")
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_TTL_SECS);

        let admin_email =
            env_nonempty("IAM_ADMIN_EMAIL").unwrap_or_else(|| DEFAULT_ADMIN_EMAIL.to_owned());

        let admin_password = match env_nonempty("IAM_ADMIN_PASSWORD") {
            Some(password) => password,
            None => {
                let generated = random_hex(16);
                eprintln!(
                    "WARN [iam]: IAM_ADMIN_PASSWORD unset; seeding admin '{admin_email}' with a \
                     generated dev password: {generated}"
                );
                generated
            }
        };

        let public_base_url =
            env_nonempty("APP_PUBLIC_URL").unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());

        Self {
            jwt_secret,
            token_ttl_secs,
            admin_email,
            admin_password,
            public_base_url,
        }
    }

    /// Build the seeded admin user, hashing the configured admin password. The
    /// plaintext password never leaves this call. The seed admin is pre-verified.
    pub fn seed_admin(&self, hasher: &dyn PasswordHasher) -> Result<User, IamError> {
        let password_hash = hasher.hash(&self.admin_password)?;
        Ok(User {
            id: Uuid::new_v4(),
            email: self.admin_email.to_lowercase(),
            password_hash,
            role: Role::Admin,
            verified: true,
            active: true,
            created_at: chrono::Utc::now(),
        })
    }
}

/// Whether the process is running in production (`APP_ENV`/`RUST_ENV`).
pub(crate) fn is_production() -> bool {
    let env = env_nonempty("APP_ENV")
        .or_else(|| env_nonempty("RUST_ENV"))
        .unwrap_or_default()
        .to_lowercase();
    matches!(env.as_str(), "production" | "prod")
}

/// Resolve the JWT signing secret. Pure (no env access) so the dev/prod policy is
/// unit-testable: provided → use it; prod + missing → error (fail closed); dev +
/// missing → an ephemeral random secret (with a warning).
fn resolve_jwt_secret(is_production: bool, provided: Option<Vec<u8>>) -> Result<Vec<u8>, String> {
    match (provided, is_production) {
        (Some(secret), _) => Ok(secret),
        (None, true) => Err(
            "IAM_JWT_SECRET must be set in production (APP_ENV/RUST_ENV=production); refusing \
             to start with a random secret."
                .to_owned(),
        ),
        (None, false) => {
            eprintln!(
                "WARN [iam]: IAM_JWT_SECRET unset; using an ephemeral random dev secret \
                 (tokens won't survive a restart). Set IAM_JWT_SECRET in real deployments."
            );
            Ok(random_bytes(32))
        }
    }
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn random_bytes(len: usize) -> Vec<u8> {
    let mut rng = OsRng;
    let mut buffer = vec![0u8; len];
    rng.fill_bytes(&mut buffer);
    buffer
}

fn random_hex(byte_len: usize) -> String {
    let mut out = String::with_capacity(byte_len * 2);
    for byte in random_bytes(byte_len) {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::resolve_jwt_secret;

    #[test]
    fn provided_secret_is_used_in_any_environment() {
        assert_eq!(
            resolve_jwt_secret(true, Some(b"abc".to_vec())).unwrap(),
            b"abc"
        );
        assert_eq!(
            resolve_jwt_secret(false, Some(b"abc".to_vec())).unwrap(),
            b"abc"
        );
    }

    #[test]
    fn missing_secret_fails_closed_in_production() {
        assert!(resolve_jwt_secret(true, None).is_err());
    }

    #[test]
    fn missing_secret_falls_back_to_random_in_dev() {
        let secret = resolve_jwt_secret(false, None).expect("dev fallback");
        assert_eq!(secret.len(), 32);
    }
}
