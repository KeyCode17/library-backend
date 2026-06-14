//! Gateway composition root.
//!
//! The gateway is the only binary context (per ADR 0002): it constructs concrete
//! infrastructure adapters, injects them into each context's use cases, and merges
//! the context routers into one application. Health, the public catalog, IAM
//! auth/RBAC, and the lending loan lifecycle are wired here.

pub mod presentation;

mod catalog_bridge;

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

use lending::application::{ApproveLoan, BorrowBook, ListLoans, ReturnLoan};
use lending::domain::{BookGateway, Clock, LoanRepository};
use lending::infrastructure::{InMemoryLoanRepository, SystemClock};
use lending::presentation::LendingState;

use catalog_bridge::CatalogBookGateway;

/// Build the application router with all contexts composed in.
///
/// In-memory adapters stand in for the Postgres adapters for now. The catalog
/// book store is shared between catalog (reads) and lending (availability writes,
/// via the bridge), so a borrow is reflected by `GET /books`.
pub fn router() -> Router {
    let books: Arc<dyn BookRepository> = Arc::new(InMemoryBookRepository::seeded());
    let iam_state = build_iam_state();

    Router::new()
        .merge(presentation::health::routes())
        .merge(catalog_router(books.clone()))
        .merge(iam::presentation::router(iam_state.clone()))
        .merge(lending_router(books, iam_state.tokens.clone()))
}

/// Public, read-only catalog (no auth — deliberately).
fn catalog_router(books: Arc<dyn BookRepository>) -> Router {
    let state = CatalogState {
        list_books: Arc::new(ListBooks::new(books.clone())),
        get_book: Arc::new(GetBook::new(books)),
    };
    catalog::presentation::router(state)
}

/// IAM: auth + RBAC. Secrets and the seed admin come from config/env.
fn build_iam_state() -> IamState {
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

    IamState {
        register: Arc::new(RegisterUser::new(users.clone(), hasher.clone())),
        login: Arc::new(LoginUser::new(
            users.clone(),
            hasher.clone(),
            tokens.clone(),
        )),
        current_user: Arc::new(GetCurrentUser::new(users.clone())),
        assign_role: Arc::new(AssignRole::new(users)),
        tokens,
    }
}

/// Lending: the loan lifecycle. All routes require a bearer token; the bridge
/// connects loans to catalog book availability.
fn lending_router(books: Arc<dyn BookRepository>, tokens: Arc<dyn TokenService>) -> Router {
    let loans: Arc<dyn LoanRepository> = Arc::new(InMemoryLoanRepository::new());
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let book_gateway: Arc<dyn BookGateway> = Arc::new(CatalogBookGateway::new(books));

    let state = LendingState {
        borrow: Arc::new(BorrowBook::new(
            loans.clone(),
            book_gateway.clone(),
            clock.clone(),
        )),
        return_loan: Arc::new(ReturnLoan::new(loans.clone(), book_gateway, clock.clone())),
        approve: Arc::new(ApproveLoan::new(loans.clone(), clock)),
        list: Arc::new(ListLoans::new(loans)),
        tokens,
    };
    lending::presentation::router(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    const SEEDED_BOOK: &str = "00000000-0000-4000-8000-000000000001";

    async fn call(
        app: &Router,
        method: &str,
        uri: &str,
        bearer: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(token) = bearer {
            builder = builder.header(AUTHORIZATION, format!("Bearer {token}"));
        }
        let request = match body {
            Some(value) => builder
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(value.to_string()))
                .expect("request builds"),
            None => builder.body(Body::empty()).expect("request builds"),
        };
        let response = app.clone().oneshot(request).await.expect("responds");
        let status = response.status();
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).expect("json")
        };
        (status, value)
    }

    #[tokio::test]
    async fn catalog_public_iam_and_lending_protected() {
        let app = router();
        assert_eq!(
            call(&app, "GET", "/books", None, None).await.0,
            StatusCode::OK
        );
        assert_eq!(
            call(&app, "GET", "/auth/me", None, None).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            call(&app, "GET", "/loans", None, None).await.0,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn borrowing_flips_catalog_availability_end_to_end() {
        let app = router();

        // Register + login a member.
        call(
            &app,
            "POST",
            "/auth/register",
            None,
            Some(json!({"email": "reader@example.com", "password": "password123"})),
        )
        .await;
        let (_, token_body) = call(
            &app,
            "POST",
            "/auth/login",
            None,
            Some(json!({"email": "reader@example.com", "password": "password123"})),
        )
        .await;
        let token = token_body["token"].as_str().expect("token").to_owned();

        // The seeded book starts available.
        let (_, before) = call(&app, "GET", &format!("/books/{SEEDED_BOOK}"), None, None).await;
        assert_eq!(before["available"], true);

        // Borrow it.
        let (status, _) = call(
            &app,
            "POST",
            "/loans",
            Some(&token),
            Some(json!({"book_id": SEEDED_BOOK})),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // Catalog now reflects it as unavailable.
        let (_, after) = call(&app, "GET", &format!("/books/{SEEDED_BOOK}"), None, None).await;
        assert_eq!(after["available"], false);
    }
}
