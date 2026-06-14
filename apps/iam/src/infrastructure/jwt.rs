//! JWT (HS256) token service. The signing secret is injected — never hardcoded.

use jsonwebtoken::{
    decode, encode, get_current_timestamp, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{AuthPrincipal, IamError, IssuedToken, Role, TokenService};

/// JWT claims. `sub` is the user id, `role` the wire role string.
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    role: String,
    iat: u64,
    exp: u64,
}

/// Issues and verifies HS256 JWTs.
pub struct JwtTokenService {
    encoding: EncodingKey,
    decoding: DecodingKey,
    validation: Validation,
    ttl_secs: u64,
}

impl JwtTokenService {
    /// Build a service from the raw signing secret and token lifetime.
    pub fn new(secret: &[u8], ttl_secs: u64) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
            validation: Validation::new(Algorithm::HS256),
            ttl_secs,
        }
    }
}

impl TokenService for JwtTokenService {
    fn issue(&self, principal: &AuthPrincipal) -> Result<IssuedToken, IamError> {
        let now = get_current_timestamp();
        let claims = Claims {
            sub: principal.user_id.to_string(),
            role: principal.role.as_str().to_owned(),
            iat: now,
            exp: now + self.ttl_secs,
        };
        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|error| IamError::Token(error.to_string()))?;
        Ok(IssuedToken {
            token,
            expires_in_secs: self.ttl_secs as i64,
        })
    }

    fn verify(&self, token: &str) -> Result<AuthPrincipal, IamError> {
        // Any failure (bad signature, expired, malformed) is a flat Unauthorized;
        // the cause is not leaked to the caller.
        let data = decode::<Claims>(token, &self.decoding, &self.validation)
            .map_err(|_| IamError::Unauthorized)?;
        let user_id = Uuid::parse_str(&data.claims.sub).map_err(|_| IamError::Unauthorized)?;
        let role = Role::parse(&data.claims.role).ok_or(IamError::Unauthorized)?;
        Ok(AuthPrincipal { user_id, role })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> JwtTokenService {
        JwtTokenService::new(b"test-secret-not-a-real-key", 3600)
    }

    fn principal() -> AuthPrincipal {
        AuthPrincipal {
            user_id: Uuid::new_v4(),
            role: Role::Admin,
        }
    }

    #[test]
    fn issued_token_verifies_back_to_the_principal() {
        let svc = service();
        let p = principal();
        let issued = svc.issue(&p).expect("issue");

        let verified = svc.verify(&issued.token).expect("verify");
        assert_eq!(verified, p);
        assert_eq!(issued.expires_in_secs, 3600);
    }

    #[test]
    fn tokens_signed_with_another_secret_are_rejected() {
        let issued = service().issue(&principal()).expect("issue");
        let other = JwtTokenService::new(b"a-different-secret", 3600);
        assert!(matches!(
            other.verify(&issued.token),
            Err(IamError::Unauthorized)
        ));
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(matches!(
            service().verify("not.a.jwt"),
            Err(IamError::Unauthorized)
        ));
    }
}
