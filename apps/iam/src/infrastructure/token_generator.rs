//! Random token generator: 256-bit tokens, stored as their hex SHA-256.

use std::fmt::Write as _;

use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};

use crate::domain::TokenGenerator;

pub struct RandomTokenGenerator;

impl RandomTokenGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RandomTokenGenerator {
    fn default() -> Self {
        Self::new()
    }
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

impl TokenGenerator for RandomTokenGenerator {
    fn generate(&self) -> (String, String) {
        let mut rng = OsRng;
        let mut raw = [0u8; 32];
        rng.fill_bytes(&mut raw);
        let raw_hex = to_hex(&raw);
        let hash = self.hash(&raw_hex);
        (raw_hex, hash)
    }

    fn hash(&self, raw: &str) -> String {
        to_hex(&Sha256::digest(raw.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_yields_distinct_raw_tokens_with_matching_hash() {
        let generator = RandomTokenGenerator::new();
        let (raw_a, hash_a) = generator.generate();
        let (raw_b, _hash_b) = generator.generate();

        assert_ne!(raw_a, raw_b, "tokens must be random");
        assert_eq!(generator.hash(&raw_a), hash_a, "hash must be reproducible");
        assert_ne!(raw_a, hash_a, "stored hash must differ from the raw token");
    }
}
