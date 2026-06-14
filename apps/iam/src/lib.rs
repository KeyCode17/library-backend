//! IAM bounded context: authentication, roles, and permissions.
//!
//! Hexagonal layering (ADR 0002):
//! - `domain` — `User`, `Role`/`Permission` (RBAC), `AuthPrincipal`, the
//!   `UserRepository`/`PasswordHasher`/`TokenService` ports, and `IamError`.
//!   Pure: no argon2/jwt/axum imports.
//! - `application` — use cases: register, login, current-user, assign-role.
//! - `infrastructure` — Argon2 hasher, JWT token service, in-memory user store,
//!   and env-driven config. Secrets come from config, never hardcoded.
//! - `presentation` — the HTTP router, the bearer-token extractor, and DTOs.
//!
//! Security posture: passwords are Argon2-hashed and never stored or serialized
//! in plaintext; the JWT signing secret is config-driven; authorization is
//! enforced server-side in the use cases (the authority), not just the edge.

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;
