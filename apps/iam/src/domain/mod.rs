//! Domain layer: entities, RBAC value objects, ports, and errors. Pure.

pub mod email_token;
pub mod error;
pub mod pagination;
pub mod ports;
pub mod principal;
pub mod role;
pub mod user;

pub use email_token::{EmailToken, EmailTokenKind};
pub use error::IamError;
pub use pagination::{Page, PageRequest};
pub use ports::{
    Clock, EmailSender, EmailTokenRepository, IssuedToken, PasswordHasher, TokenGenerator,
    TokenService, UserRepository,
};
pub use principal::AuthPrincipal;
pub use role::{Permission, Role};
pub use user::User;
