//! Gateway composition root.
//!
//! The gateway is the only binary context (per ADR 0002): it assembles the HTTP
//! router from the feature contexts (`iam`, `catalog`, `lending`, ...) as they
//! come online. The M0 skeleton wires nothing but the health probe.

pub mod presentation;

use axum::Router;

/// Build the application router.
///
/// Feature routers are merged here as contexts land; today it is health only.
pub fn router() -> Router {
    Router::new().merge(presentation::health::routes())
}
