//! Lending bounded context: the loan lifecycle (borrow → due → return → approve).
//!
//! Hexagonal layering (ADR 0002):
//! - `domain` — the `Loan` entity (which owns its state transitions), `LoanStatus`,
//!   pagination value objects, the `LoanRepository` / `BookGateway` / `Clock`
//!   ports, and `LendingError`. Pure: no `catalog`, no framework imports.
//! - `application` — use cases: borrow, return, approve, list. Authorization is
//!   enforced here (the server-side authority), reusing the IAM RBAC model.
//! - `infrastructure` — in-memory loan store and the system clock.
//! - `presentation` — the HTTP router (reusing IAM's bearer extractor) and DTOs.
//!
//! Book availability is reached through the `BookGateway` port, so lending stays
//! decoupled from `catalog` at the domain level; the gateway composes the bridge.

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;
