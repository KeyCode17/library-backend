use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::dto::{BookDto, BookListResponse, ErrorBody};
use crate::application::{GetBook, ListBooks};
use crate::domain::pagination::DEFAULT_PAGE_SIZE;
use crate::domain::{BookFilter, PageRequest};

/// State injected into the catalog routes: one use case per operation. Cheap to
/// clone (just `Arc`s), as Axum requires.
#[derive(Clone)]
pub struct CatalogState {
    pub list_books: Arc<ListBooks>,
    pub get_book: Arc<GetBook>,
}

/// Query params for `GET /books`: pagination plus the optional shelf/row/ISBN
/// finder.
#[derive(Debug, Deserialize)]
struct ListQuery {
    page: Option<u32>,
    page_size: Option<u32>,
    shelf: Option<String>,
    row: Option<i32>,
    isbn: Option<String>,
}

/// `GET /books` — public catalog listing in the `{ data, pagination }` envelope,
/// optionally narrowed by the shelf/row finder.
async fn list_books(State(state): State<CatalogState>, Query(query): Query<ListQuery>) -> Response {
    let request = PageRequest::new(
        query.page.unwrap_or(1),
        query.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
    );
    let filter = BookFilter {
        shelf: query.shelf,
        row: query.row,
        isbn: query.isbn,
    };

    match state.list_books.execute(&filter, request).await {
        Ok(page) => Json(BookListResponse::from(page)).into_response(),
        Err(_error) => internal_error(),
    }
}

/// `GET /books/{id}` — public book detail; `404` when there is no such book.
async fn get_book(State(state): State<CatalogState>, Path(id): Path<Uuid>) -> Response {
    match state.get_book.execute(id).await {
        Ok(Some(book)) => Json(BookDto::from(book)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorBody::new("not_found", "book not found")),
        )
            .into_response(),
        Err(_error) => internal_error(),
    }
}

fn internal_error() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorBody::new("internal", "internal error")),
    )
        .into_response()
}

/// Mount the catalog routes with the use cases as state.
pub fn router(state: CatalogState) -> Router {
    Router::new()
        .route("/books", get(list_books))
        .route("/books/{id}", get(get_book))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::BookRepository;
    use crate::infrastructure::in_memory::InMemoryBookRepository;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    fn app() -> Router {
        let repository: Arc<dyn BookRepository> = Arc::new(InMemoryBookRepository::seeded());
        let state = CatalogState {
            list_books: Arc::new(ListBooks::new(repository.clone())),
            get_book: Arc::new(GetBook::new(repository)),
        };
        router(state)
    }

    async fn get(uri: &str) -> (StatusCode, Value) {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        let status = response.status();
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("body collects")
            .to_bytes();
        let json = serde_json::from_slice(&bytes).expect("valid json");
        (status, json)
    }

    #[tokio::test]
    async fn list_returns_200_with_data_and_pagination() {
        let (status, json) = get("/books").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["data"].as_array().expect("data is array").len(), 8);
        assert_eq!(json["pagination"]["total"], 8);
        assert_eq!(json["pagination"]["page_size"], 20);
    }

    #[tokio::test]
    async fn finder_filters_by_shelf_and_row() {
        let (status, json) = get("/books?shelf=Tech").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["pagination"]["total"], 3);
        assert!(json["data"]
            .as_array()
            .expect("data is array")
            .iter()
            .all(|book| book["shelf"] == "Tech"));

        let (_, narrowed) = get("/books?shelf=Tech&row=4").await;
        assert_eq!(narrowed["pagination"]["total"], 1);
        assert_eq!(
            narrowed["data"][0]["title"],
            "The Rust Programming Language"
        );
    }

    #[tokio::test]
    async fn finder_returns_empty_page_when_nothing_matches() {
        let (status, json) = get("/books?shelf=Nowhere").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["pagination"]["total"], 0);
        assert_eq!(json["data"].as_array().expect("data is array").len(), 0);
    }

    #[tokio::test]
    async fn detail_returns_the_book() {
        let (status, json) = get("/books/00000000-0000-4000-8000-000000000002").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["id"], "00000000-0000-4000-8000-000000000002");
        assert_eq!(json["title"], "Clean Code");
        assert!(json["available"].is_boolean());
    }

    #[tokio::test]
    async fn detail_returns_404_error_body_when_absent() {
        let (status, json) = get("/books/ffffffff-ffff-4fff-8fff-ffffffffffff").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json["code"], "not_found");
        assert!(json["message"].is_string());
    }
}
