//! `POST /recommend` — a composition endpoint (FSD §3: "invoked by gateway").
//!
//! Public: preferences come in the body, and candidates are either supplied or
//! taken from the public catalog. It calls the pure `recommender` crate directly
//! (ADR 0005) — the same logic the phone runs on-device via `recommender-ffi`.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use catalog::domain::{BookFilter, BookRepository, PageRequest};
use recommender::{CandidateBook, Preferences, Recommender};

/// Largest catalog page pulled when ranking server-fetched candidates.
const CATALOG_CANDIDATE_LIMIT: u32 = 100;

#[derive(Clone)]
pub struct RecommendState {
    pub recommender: Arc<Recommender>,
    pub books: Arc<dyn BookRepository>,
}

#[derive(Debug, Default, Deserialize)]
struct PreferencesBody {
    #[serde(default)]
    preferred_shelves: Vec<String>,
    #[serde(default)]
    preferred_authors: Vec<String>,
    #[serde(default)]
    available_only: bool,
}

#[derive(Debug, Deserialize)]
struct CandidateBody {
    id: Uuid,
    shelf: String,
    author: String,
    available: bool,
}

#[derive(Debug, Deserialize)]
struct RecommendRequest {
    #[serde(default)]
    preferences: PreferencesBody,
    /// Explicit candidates to rank. Omit to rank the server's catalog.
    candidates: Option<Vec<CandidateBody>>,
}

#[derive(Debug, Serialize)]
struct RecommendResponse {
    ranked: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: &'static str,
    message: &'static str,
}

async fn recommend(
    State(state): State<RecommendState>,
    Json(body): Json<RecommendRequest>,
) -> Response {
    let preferences = Preferences {
        preferred_shelves: body.preferences.preferred_shelves,
        preferred_authors: body.preferences.preferred_authors,
        available_only: body.preferences.available_only,
    };

    let candidates = match body.candidates {
        Some(list) => list
            .into_iter()
            .map(|candidate| CandidateBook {
                id: candidate.id,
                shelf: candidate.shelf,
                author: candidate.author,
                available: candidate.available,
            })
            .collect(),
        None => match fetch_catalog_candidates(state.books.as_ref()).await {
            Ok(candidates) => candidates,
            Err(response) => return response,
        },
    };

    let ranked = state.recommender.rank(&preferences, &candidates);
    Json(RecommendResponse { ranked }).into_response()
}

async fn fetch_catalog_candidates(
    books: &dyn BookRepository,
) -> Result<Vec<CandidateBook>, Response> {
    let page = books
        .list(
            &BookFilter::default(),
            PageRequest::new(1, CATALOG_CANDIDATE_LIMIT),
        )
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    code: "internal",
                    message: "internal error",
                }),
            )
                .into_response()
        })?;

    Ok(page
        .items
        .into_iter()
        .map(|book| CandidateBook {
            id: book.id,
            shelf: book.shelf,
            author: book.author,
            available: book.available,
        })
        .collect())
}

/// Mount the public `/recommend` route.
pub fn routes(state: RecommendState) -> Router {
    Router::new()
        .route("/recommend", post(recommend))
        .with_state(state)
}
