//! Catalog bounded context: books and their physical shelf location.
//!
//! Hexagonal layering (ADR 0002):
//! - `domain` — the `Book` entity, the `BookRepository` port, pagination value
//!   objects, and domain errors. No framework imports.
//! - `application` — use cases (`ListBooks`) orchestrating the port.
//! - `infrastructure` — adapters implementing the port (in-memory seed today;
//!   the Postgres/SeaORM books table lives in the `migration` crate).
//! - `presentation` — the HTTP router and DTOs the gateway merges.
//!
//! Dependency rule: presentation → application → domain; infrastructure
//! implements domain ports. The gateway is the composition root that injects a
//! concrete repository.

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;
