//! Domain layer: entities, RBAC value objects, ports, and errors. Pure.

pub mod error;
pub mod ports;
pub mod principal;
pub mod role;
pub mod user;

pub use error::IamError;
pub use ports::{IssuedToken, PasswordHasher, TokenService, UserRepository};
pub use principal::AuthPrincipal;
pub use role::{Permission, Role};
pub use user::User;
