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

pub struct IamConfig {
    pub jwt_secret: Vec<u8>,
    pub token_ttl_secs: u64,
    pub admin_email: String,
    pub admin_password: String,
}

impl IamConfig {
    /// Read config from the environment, applying safe dev fallbacks with loud
    /// warnings. Real deployments must set `IAM_JWT_SECRET` (and usually
    /// `IAM_ADMIN_EMAIL` / `IAM_ADMIN_PASSWORD`).
    pub fn from_env() -> Self {
        let jwt_secret = match env_nonempty("IAM_JWT_SECRET") {
            Some(secret) => secret.into_bytes(),
            None => {
                eprintln!(
                    "WARN [iam]: IAM_JWT_SECRET unset; using an ephemeral random dev secret \
                     (tokens won't survive a restart). Set IAM_JWT_SECRET in real deployments."
                );
                random_bytes(32)
            }
        };

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

        Self {
            jwt_secret,
            token_ttl_secs,
            admin_email,
            admin_password,
        }
    }

    /// Build the seeded admin user, hashing the configured admin password. The
    /// plaintext password never leaves this call.
    pub fn seed_admin(&self, hasher: &dyn PasswordHasher) -> Result<User, IamError> {
        let password_hash = hasher.hash(&self.admin_password)?;
        Ok(User::new(
            Uuid::new_v4(),
            self.admin_email.to_lowercase(),
            password_hash,
            Role::Admin,
        ))
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
