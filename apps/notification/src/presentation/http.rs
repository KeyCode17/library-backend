use std::sync::Arc;

use axum::extract::{FromRef, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use iam::domain::TokenService;
use iam::presentation::AuthenticatedUser;

use super::dto::{DeviceRegistrationRequest, DeviceResponse, ErrorBody, NotificationListResponse};
use crate::application::{ListNotifications, RegisterDevice};
use crate::domain::pagination::DEFAULT_PAGE_SIZE;
use crate::domain::{NotificationError, PageRequest};

/// State for the notification REST routes.
#[derive(Clone)]
pub struct NotificationState {
    pub register_device: Arc<RegisterDevice>,
    pub list_notifications: Arc<ListNotifications>,
    pub tokens: Arc<dyn TokenService>,
}

/// Lets IAM's bearer extractor pull the token service.
impl FromRef<NotificationState> for Arc<dyn TokenService> {
    fn from_ref(state: &NotificationState) -> Self {
        state.tokens.clone()
    }
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    page: Option<u32>,
    page_size: Option<u32>,
}

/// `POST /notifications/devices` — register an FCM token for the caller.
async fn register_device(
    State(state): State<NotificationState>,
    AuthenticatedUser(principal): AuthenticatedUser,
    Json(body): Json<DeviceRegistrationRequest>,
) -> Response {
    match state
        .register_device
        .execute(principal.user_id, body.token, body.platform)
        .await
    {
        Ok(device) => (StatusCode::CREATED, Json(DeviceResponse::from(device))).into_response(),
        Err(error) => error_response(&error),
    }
}

/// `GET /notifications` — the caller's reminder history.
async fn list_notifications(
    State(state): State<NotificationState>,
    AuthenticatedUser(principal): AuthenticatedUser,
    Query(query): Query<ListQuery>,
) -> Response {
    let request = PageRequest::new(
        query.page.unwrap_or(1),
        query.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
    );
    match state
        .list_notifications
        .execute(principal.user_id, request)
        .await
    {
        Ok(page) => Json(NotificationListResponse::from(page)).into_response(),
        Err(error) => error_response(&error),
    }
}

fn error_response(error: &NotificationError) -> Response {
    let (status, code, message) = match error {
        NotificationError::InvalidToken => (
            StatusCode::BAD_REQUEST,
            "invalid_token",
            "device token is empty",
        ),
        NotificationError::Repository(_)
        | NotificationError::Dependency(_)
        | NotificationError::Push(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "internal error",
        ),
    };
    (status, Json(ErrorBody::new(code, message))).into_response()
}

/// Mount the notification routes (both require a bearer token).
pub fn router(state: NotificationState) -> Router {
    Router::new()
        .route("/notifications/devices", post(register_device))
        .route("/notifications", get(list_notifications))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::RegisterDevice;
    use crate::domain::Clock;
    use crate::infrastructure::{
        InMemoryDeviceRepository, InMemoryReminderRepository, SystemClock,
    };
    use axum::body::Body;
    use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
    use axum::http::Request;
    use http_body_util::BodyExt;
    use iam::domain::{AuthPrincipal, Role};
    use iam::infrastructure::jwt::JwtTokenService;
    use serde_json::{json, Value};
    use tower::ServiceExt;
    use uuid::Uuid;

    fn build() -> (Router, Arc<dyn TokenService>) {
        let tokens: Arc<dyn TokenService> =
            Arc::new(JwtTokenService::new(b"notif-test-secret", 3600));
        let devices = Arc::new(InMemoryDeviceRepository::new());
        let reminders = Arc::new(InMemoryReminderRepository::new());
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);

        let state = NotificationState {
            register_device: Arc::new(RegisterDevice::new(devices, clock)),
            list_notifications: Arc::new(ListNotifications::new(reminders)),
            tokens: tokens.clone(),
        };
        (router(state), tokens)
    }

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
                .expect("request"),
            None => builder.body(Body::empty()).expect("request"),
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
    async fn device_registration_requires_auth() {
        let (app, _) = build();
        let (status, _) = call(
            &app,
            "POST",
            "/notifications/devices",
            None,
            Some(json!({"token": "abc", "platform": "android"})),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn registers_a_device_for_the_authenticated_user() {
        let (app, tokens) = build();
        let user = Uuid::new_v4();
        let token = tokens
            .issue(&AuthPrincipal {
                user_id: user,
                role: Role::Member,
            })
            .expect("issue")
            .token;

        let (status, body) = call(
            &app,
            "POST",
            "/notifications/devices",
            Some(&token),
            Some(json!({"token": "fcm-device-token", "platform": "android"})),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["user_id"], user.to_string());
        assert_eq!(body["token"], "fcm-device-token");
        assert_eq!(body["platform"], "android");
    }

    #[tokio::test]
    async fn notification_history_requires_auth() {
        let (app, _) = build();
        let (status, _) = call(&app, "GET", "/notifications", None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
