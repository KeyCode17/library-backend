//! Gateway composition root.
//!
//! The gateway is the only binary context (per ADR 0002): it constructs concrete
//! infrastructure adapters, injects them into each context's use cases, and merges
//! the context routers into one application. The M0 health probe plus the T-001
//! catalog listing are wired here.

pub mod presentation;

use std::sync::Arc;

use axum::Router;
use catalog::application::{GetBook, ListBooks};
use catalog::domain::BookRepository;
use catalog::infrastructure::in_memory::InMemoryBookRepository;
use catalog::presentation::CatalogState;

/// Build the application router with all contexts composed in.
///
/// The catalog uses an in-memory seeded repository for now (the Postgres adapter
/// lands with the DB wiring); swapping it is a one-line change here, the only
/// place that knows the concrete adapter.
pub fn router() -> Router {
    let book_repository: Arc<dyn BookRepository> = Arc::new(InMemoryBookRepository::seeded());
    let catalog_state = CatalogState {
        list_books: Arc::new(ListBooks::new(book_repository.clone())),
        get_book: Arc::new(GetBook::new(book_repository)),
    };

    Router::new()
        .merge(presentation::health::routes())
        .merge(catalog::presentation::router(catalog_state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

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

        assert!(body.contains("\"data\""));
        assert!(body.contains("\"pagination\""));
        assert!(body.contains("Clean Code"));
    }
}
