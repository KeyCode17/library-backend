use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use super::dto::{BookListResponse, ErrorBody};
use crate::application::ListBooks;
use crate::domain::pagination::DEFAULT_PAGE_SIZE;
use crate::domain::PageRequest;

/// Query params for `GET /books`. Both optional; defaults applied below.
#[derive(Debug, Deserialize)]
struct ListQuery {
    page: Option<u32>,
    page_size: Option<u32>,
}

/// `GET /books` — public catalog listing in the `{ data, pagination }` envelope.
async fn list_books(
    State(use_case): State<Arc<ListBooks>>,
    Query(query): Query<ListQuery>,
) -> Response {
    let request = PageRequest::new(
        query.page.unwrap_or(1),
        query.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
    );

    match use_case.execute(request).await {
        Ok(page) => Json(BookListResponse::from(page)).into_response(),
        Err(_error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody {
                error: "internal error",
            }),
        )
            .into_response(),
    }
}

/// Mount the catalog routes with the `ListBooks` use case as state.
pub fn router(use_case: Arc<ListBooks>) -> Router {
    Router::new()
        .route("/books", get(list_books))
        .with_state(use_case)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::BookRepository;
    use crate::infrastructure::in_memory::InMemoryBookRepository;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn app() -> Router {
        let repository: Arc<dyn BookRepository> = Arc::new(InMemoryBookRepository::seeded());
        router(Arc::new(ListBooks::new(repository)))
    }

    #[tokio::test]
    async fn get_books_returns_200_with_data_and_pagination() {
        let response = app()
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
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("valid json");

        assert_eq!(json["data"].as_array().expect("data is array").len(), 8);
        assert_eq!(json["pagination"]["total"], 8);
        assert_eq!(json["pagination"]["page"], 1);
        assert_eq!(json["pagination"]["page_size"], 20);
        assert_eq!(json["pagination"]["total_pages"], 1);

        let first = &json["data"][0];
        assert!(first["id"].is_string());
        assert!(first["title"].is_string());
        assert!(first["row"].is_number());
        assert!(first["available"].is_boolean());
    }

    #[tokio::test]
    async fn get_books_honours_page_and_page_size() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/books?page=2&page_size=3")
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
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("valid json");

        assert_eq!(json["data"].as_array().expect("data is array").len(), 3);
        assert_eq!(json["pagination"]["page"], 2);
        assert_eq!(json["pagination"]["page_size"], 3);
        assert_eq!(json["pagination"]["total"], 8);
        assert_eq!(json["pagination"]["total_pages"], 3);
    }
}
