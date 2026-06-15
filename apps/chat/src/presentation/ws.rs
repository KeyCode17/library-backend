//! The WebSocket endpoint: authenticate the upgrade, then bridge the socket to
//! the room's broadcast channel.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;

use iam::domain::AuthPrincipal;

use super::dto::{ChatMessageDto, ChatSendDto};
use super::http::ChatState;

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    room: String,
    token: Option<String>,
}

/// `GET /ws/chat?room=<room>&token=<jwt>` — authenticate, then upgrade.
///
/// Auth, in order: a `token` query param (a browser cannot set headers on a WS
/// handshake), `Authorization: Bearer <jwt>` for non-browser clients, or the
/// `session` cookie (web, which holds the JWT in an httpOnly cookie with no JS
/// token to put in the query param). Unauthenticated upgrades are rejected with
/// `401` before the protocol switch.
pub async fn upgrade(
    State(state): State<ChatState>,
    Query(query): Query<WsQuery>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let Some(token) = query
        .token
        .clone()
        .or_else(|| bearer(&headers))
        .or_else(|| session_cookie(&headers))
    else {
        return (StatusCode::UNAUTHORIZED, "missing token").into_response();
    };
    let Ok(principal) = state.tokens.verify(&token) else {
        return (StatusCode::UNAUTHORIZED, "invalid token").into_response();
    };

    let room = query.room.clone();
    ws.on_upgrade(move |socket| serve_socket(socket, state, room, principal))
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(|token| token.trim().to_owned())
        .filter(|token| !token.is_empty())
}

/// Extract the JWT from the `session` cookie set by `POST /auth/login`.
fn session_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|value| value.to_str().ok())?
        .split(';')
        .map(str::trim)
        .find_map(|cookie| cookie.strip_prefix("session="))
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
}

async fn serve_socket(socket: WebSocket, state: ChatState, room: String, principal: AuthPrincipal) {
    let mut broadcast_rx = state.hub.subscribe(&room);
    let (mut sink, mut stream) = socket.split();

    // Forward this room's broadcast messages to the client.
    let mut forward = tokio::spawn(async move {
        while let Ok(message) = broadcast_rx.recv().await {
            let Ok(json) = serde_json::to_string(&ChatMessageDto::from(message)) else {
                continue;
            };
            if sink.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    // Persist + broadcast each message the client sends.
    let post = state.post_message.clone();
    let mut ingest = tokio::spawn(async move {
        while let Some(Ok(message)) = stream.next().await {
            if let Message::Text(text) = message {
                if let Ok(send) = serde_json::from_str::<ChatSendDto>(text.as_str()) {
                    let _ = post
                        .execute(room.clone(), principal.user_id, send.body)
                        .await;
                }
            }
        }
    });

    // When either half ends, stop the other.
    tokio::select! {
        _ = &mut forward => ingest.abort(),
        _ = &mut ingest => forward.abort(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message as ClientMessage;
    use uuid::Uuid;

    use iam::domain::{AuthPrincipal, Role, TokenService};
    use iam::infrastructure::jwt::JwtTokenService;

    use crate::application::{ListHistory, PostMessage};
    use crate::domain::{Clock, MessageBroadcaster, MessageRepository, PageRequest};
    use crate::infrastructure::{InMemoryMessageRepository, RoomHub, SystemClock};
    use crate::presentation::http::{router, ChatState};

    struct Server {
        addr: String,
        tokens: Arc<dyn TokenService>,
        messages: Arc<InMemoryMessageRepository>,
    }

    async fn spawn() -> Server {
        let tokens: Arc<dyn TokenService> =
            Arc::new(JwtTokenService::new(b"chat-test-secret", 3600));
        let messages = Arc::new(InMemoryMessageRepository::new());
        let hub = Arc::new(RoomHub::new());
        let repo: Arc<dyn MessageRepository> = messages.clone();
        let broadcaster: Arc<dyn MessageBroadcaster> = hub.clone();
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);

        let state = ChatState {
            post_message: Arc::new(PostMessage::new(repo.clone(), broadcaster, clock)),
            history: Arc::new(ListHistory::new(repo)),
            hub,
            tokens: tokens.clone(),
        };

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        tokio::spawn(async move {
            axum::serve(listener, router(state)).await.expect("serve");
        });

        Server {
            addr,
            tokens,
            messages,
        }
    }

    fn token_for(server: &Server, user_id: Uuid) -> String {
        server
            .tokens
            .issue(&AuthPrincipal {
                user_id,
                role: Role::Member,
            })
            .expect("issue")
            .token
    }

    #[tokio::test]
    async fn message_is_broadcast_to_room_peers_and_persisted() {
        let server = spawn().await;
        let alice = Uuid::new_v4();
        let bob = Uuid::new_v4();
        let token_a = token_for(&server, alice);
        let token_b = token_for(&server, bob);

        let url_a = format!("ws://{}/ws/chat?room=lib&token={token_a}", server.addr);
        let url_b = format!("ws://{}/ws/chat?room=lib&token={token_b}", server.addr);

        let (mut conn_a, _) = tokio_tungstenite::connect_async(&url_a)
            .await
            .expect("a connects");
        let (mut conn_b, _) = tokio_tungstenite::connect_async(&url_b)
            .await
            .expect("b connects");

        // Let both sockets finish subscribing before sending.
        tokio::time::sleep(Duration::from_millis(150)).await;

        conn_a
            .send(ClientMessage::Text(r#"{"body":"hello room"}"#.into()))
            .await
            .expect("a sends");

        // Bob (a peer) receives the broadcast.
        let received = tokio::time::timeout(Duration::from_secs(3), conn_b.next())
            .await
            .expect("no timeout")
            .expect("a frame")
            .expect("ok frame");
        let text = match received {
            ClientMessage::Text(text) => text,
            other => panic!("expected text frame, got {other:?}"),
        };
        let value: serde_json::Value = serde_json::from_str(text.as_str()).expect("json");
        assert_eq!(value["body"], "hello room");
        assert_eq!(value["room"], "lib");
        assert_eq!(value["user_id"], alice.to_string());

        // It is persisted to history.
        let page = server
            .messages
            .list_by_room("lib", PageRequest::new(1, 50))
            .await
            .expect("history");
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].body, "hello room");
    }

    #[tokio::test]
    async fn unauthenticated_upgrade_is_rejected() {
        let server = spawn().await;
        let url = format!("ws://{}/ws/chat?room=lib", server.addr);
        let result = tokio_tungstenite::connect_async(&url).await;
        assert!(result.is_err(), "handshake without a token must fail");
    }

    #[tokio::test]
    async fn invalid_token_upgrade_is_rejected() {
        let server = spawn().await;
        let url = format!("ws://{}/ws/chat?room=lib&token=not.a.jwt", server.addr);
        let result = tokio_tungstenite::connect_async(&url).await;
        assert!(result.is_err(), "handshake with a bad token must fail");
    }

    #[tokio::test]
    async fn session_cookie_authenticates_the_upgrade() {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        use tokio_tungstenite::tungstenite::http::header::COOKIE;

        let server = spawn().await;
        let carol = Uuid::new_v4();
        let token = token_for(&server, carol);

        // Web has no JS token for the query param — only the httpOnly cookie.
        let url = format!("ws://{}/ws/chat?room=lib", server.addr);
        let mut request = url.into_client_request().expect("request");
        request.headers_mut().insert(
            COOKIE,
            format!("session={token}").parse().expect("cookie header"),
        );

        let (mut conn, _) = tokio_tungstenite::connect_async(request)
            .await
            .expect("cookie handshake connects");

        // Prove the principal was extracted from the cookie: a sent message is
        // persisted under carol's id.
        tokio::time::sleep(Duration::from_millis(150)).await;
        conn.send(ClientMessage::Text(r#"{"body":"via cookie"}"#.into()))
            .await
            .expect("sends");
        tokio::time::sleep(Duration::from_millis(150)).await;

        let page = server
            .messages
            .list_by_room("lib", PageRequest::new(1, 50))
            .await
            .expect("history");
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].body, "via cookie");
        assert_eq!(page.items[0].user_id, carol);
    }
}
