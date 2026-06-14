//! Liveness probe. No domain dependency — the gateway answers this itself.

use axum::{routing::get, Json, Router};
use serde::Serialize;

/// Body returned by `GET /healthz`. `status` is `"ok"` while the process serves.
#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: &'static str,
}

/// Liveness handler: reports the gateway is up. Always `200`.
pub async fn healthz() -> Json<HealthStatus> {
    Json(HealthStatus { status: "ok" })
}

/// Mount the health routes.
pub fn routes() -> Router {
    Router::new().route("/healthz", get(healthz))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn healthz_returns_200_with_ok_body() {
        let response = routes()
            .oneshot(
                Request::builder()
                    .uri("/healthz")
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
        assert_eq!(&bytes[..], br#"{"status":"ok"}"#);
    }
}
