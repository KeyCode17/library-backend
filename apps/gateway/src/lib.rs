//! Gateway composition root.
//!
//! The gateway is the only binary context (per ADR 0002): it constructs concrete
//! infrastructure adapters, injects them into each context's use cases, and merges
//! the context routers into one application. Health, the public catalog, and the
//! IAM auth/RBAC surface are wired here.

pub mod presentation;

use std::sync::Arc;

use axum::Router;

use catalog::application::{GetBook, ListBooks};
use catalog::domain::BookRepository;
use catalog::infrastructure::in_memory::InMemoryBookRepository;
use catalog::presentation::CatalogState;

use iam::application::{AssignRole, GetCurrentUser, LoginUser, RegisterUser};
use iam::domain::{PasswordHasher, TokenService, UserRepository};
use iam::infrastructure::{
    Argon2PasswordHasher, IamConfig, InMemoryUserRepository, JwtTokenService,
};
use iam::presentation::IamState;

/// Build the application router with all contexts composed in.
///
/// In-memory adapters stand in for the Postgres adapters for now; swapping them
/// is localized to the builders below — the only place that knows the concrete
/// adapter.
pub fn router() -> Router {
    Router::new()
        .merge(presentation::health::routes())
        .merge(catalog_router())
        .merge(iam_router())
}

/// Public, read-only catalog (no auth — deliberately).
fn catalog_router() -> Router {
    let book_repository: Arc<dyn BookRepository> = Arc::new(InMemoryBookRepository::seeded());
    let state = CatalogState {
        list_books: Arc::new(ListBooks::new(book_repository.clone())),
        get_book: Arc::new(GetBook::new(book_repository)),
    };
    catalog::presentation::router(state)
}

/// IAM: auth + RBAC. Secrets and the seed admin come from config/env.
fn iam_router() -> Router {
    let config = IamConfig::from_env();
    let hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2PasswordHasher::new());
    let tokens: Arc<dyn TokenService> = Arc::new(JwtTokenService::new(
        &config.jwt_secret,
        config.token_ttl_secs,
    ));

    let admin = config
        .seed_admin(hasher.as_ref())
        .expect("seed admin user at startup");
    let users: Arc<dyn UserRepository> = Arc::new(InMemoryUserRepository::seeded_with(vec![admin]));

    let state = IamState {
        register: Arc::new(RegisterUser::new(users.clone(), hasher.clone())),
        login: Arc::new(LoginUser::new(
            users.clone(),
            hasher.clone(),
            tokens.clone(),
        )),
        current_user: Arc::new(GetCurrentUser::new(users.clone())),
        assign_role: Arc::new(AssignRole::new(users)),
        tokens,
    };
    iam::presentation::router(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn status_of(uri: &str) -> StatusCode {
        router()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds")
            .status()
    }

    #[tokio::test]
    async fn books_endpoint_serves_the_seeded_catalog() {
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/books")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("body collects")
            .to_bytes();
        let body = String::from_utf8(bytes.to_vec()).expect("utf8 body");
        assert!(body.contains("Clean Code"));
    }

    #[tokio::test]
    async fn catalog_stays_public_but_iam_is_protected() {
        // Catalog needs no auth.
        assert_eq!(status_of("/books").await, StatusCode::OK);
        // The IAM principal endpoint rejects unauthenticated callers.
        assert_eq!(status_of("/auth/me").await, StatusCode::UNAUTHORIZED);
    }
}
