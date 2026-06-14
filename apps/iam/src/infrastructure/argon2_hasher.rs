//! Argon2id password hasher (the default, recommended parameters).

use argon2::password_hash::{Error as PhcError, PasswordHash, SaltString};
use argon2::{Algorithm, Argon2, Params, PasswordHasher as _, PasswordVerifier as _, Version};
use rand_core::OsRng;

use crate::domain::{IamError, PasswordHasher};

// Explicit Argon2id parameters pinned at the OWASP minimum (rather than relying
// on `Argon2::default()`): 19 MiB memory, 2 iterations, 1 lane.
const ARGON2_MEMORY_KIB: u32 = 19_456;
const ARGON2_ITERATIONS: u32 = 2;
const ARGON2_PARALLELISM: u32 = 1;

/// Hashes and verifies passwords with Argon2id. Salts come from the OS CSPRNG.
pub struct Argon2PasswordHasher {
    argon2: Argon2<'static>,
}

impl Argon2PasswordHasher {
    pub fn new() -> Self {
        let params = Params::new(
            ARGON2_MEMORY_KIB,
            ARGON2_ITERATIONS,
            ARGON2_PARALLELISM,
            None,
        )
        .expect("argon2 params are within valid bounds");
        Self {
            argon2: Argon2::new(Algorithm::Argon2id, Version::V0x13, params),
        }
    }
}

impl Default for Argon2PasswordHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordHasher for Argon2PasswordHasher {
    fn hash(&self, plaintext: &str) -> Result<String, IamError> {
        let salt = SaltString::generate(&mut OsRng);
        self.argon2
            .hash_password(plaintext.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| IamError::Hashing(error.to_string()))
    }

    fn verify(&self, plaintext: &str, phc_hash: &str) -> Result<bool, IamError> {
        let parsed =
            PasswordHash::new(phc_hash).map_err(|error| IamError::Hashing(error.to_string()))?;
        match self.argon2.verify_password(plaintext.as_bytes(), &parsed) {
            Ok(()) => Ok(true),
            Err(PhcError::Password) => Ok(false),
            Err(error) => Err(IamError::Hashing(error.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_then_verifies_the_same_password() {
        let hasher = Argon2PasswordHasher::new();
        let hash = hasher.hash("correct horse battery").expect("hash");

        assert!(hash.starts_with("$argon2"));
        assert_ne!(hash, "correct horse battery");
        assert!(hasher
            .verify("correct horse battery", &hash)
            .expect("verify"));
        assert!(!hasher.verify("wrong password", &hash).expect("verify"));
    }

    #[test]
    fn salts_make_two_hashes_of_the_same_password_differ() {
        let hasher = Argon2PasswordHasher::new();
        let a = hasher.hash("same-password").expect("hash");
        let b = hasher.hash("same-password").expect("hash");
        assert_ne!(a, b);
    }
}
