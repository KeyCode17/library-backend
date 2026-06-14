use std::sync::Arc;

use axum::extract::{FromRef, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use iam::domain::TokenService;
use iam::presentation::AuthenticatedUser;

use super::dto::{ChatMessageListResponse, ErrorBody};
use super::ws;
use crate::application::{ListHistory, PostMessage};
use crate::domain::pagination::DEFAULT_PAGE_SIZE;
use crate::domain::PageRequest;
use crate::infrastructure::RoomHub;

/// State for the chat routes. The `hub` is shared between the WebSocket handler
/// (which `subscribe`s) and `post_message` (which publishes through it).
#[derive(Clone)]
pub struct ChatState {
    pub post_message: Arc<PostMessage>,
    pub history: Arc<ListHistory>,
    pub hub: Arc<RoomHub>,
    pub tokens: Arc<dyn TokenService>,
}

/// Lets IAM's bearer extractor (REST history) pull the token service.
impl FromRef<ChatState> for Arc<dyn TokenService> {
    fn from_ref(state: &ChatState) -> Self {
        state.tokens.clone()
    }
}

#[derive(Debug, Deserialize)]
struct HistoryQuery {
    page: Option<u32>,
    page_size: Option<u32>,
}

/// `GET /chat/rooms/{room}/messages` — paginated history. Any authenticated user
/// may read a room (group chat); the extractor enforces the 401.
async fn history(
    State(state): State<ChatState>,
    _user: AuthenticatedUser,
    Path(room): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> Response {
    let request = PageRequest::new(
        query.page.unwrap_or(1),
        query.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
    );
    match state.history.execute(&room, request).await {
        Ok(page) => Json(ChatMessageListResponse::from(page)).into_response(),
        Err(_error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody::new("internal", "internal error")),
        )
            .into_response(),
    }
}

/// Mount the chat routes: REST history (bearer) and the WebSocket upgrade.
pub fn router(state: ChatState) -> Router {
    Router::new()
        .route("/chat/rooms/{room}/messages", get(history))
        .route("/ws/chat", get(ws::upgrade))
        .with_state(state)
}
