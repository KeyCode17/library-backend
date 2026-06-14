use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts, Path, State};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;

use super::dto::{
    AssignRoleRequest, AuthTokenResponse, CredentialsRequest, ErrorBody, PrincipalResponse,
};
use crate::application::{AssignRole, GetCurrentUser, LoginUser, RegisterUser};
use crate::domain::{AuthPrincipal, IamError, TokenService};

/// State injected into the IAM routes: one use case per operation plus the token
/// service (used by the bearer extractor). Cheap to clone (just `Arc`s).
#[derive(Clone)]
pub struct IamState {
    pub register: Arc<RegisterUser>,
    pub login: Arc<LoginUser>,
    pub current_user: Arc<GetCurrentUser>,
    pub assign_role: Arc<AssignRole>,
    pub tokens: Arc<dyn TokenService>,
}

/// Extractor that proves the caller presented a valid bearer token. Its presence
/// in a handler signature is what makes an endpoint require authentication;
/// absence/invalidity yields `401` before the handler runs.
pub struct AuthenticatedUser(pub AuthPrincipal);

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
    IamState: FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let iam = IamState::from_ref(state);

        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .ok_or_else(unauthorized)?;

        let principal = iam.tokens.verify(token).map_err(|_| unauthorized())?;
        Ok(AuthenticatedUser(principal))
    }
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorBody::new("unauthorized", "authentication required")),
    )
        .into_response()
}

/// `POST /auth/register` — public self-registration (creates a member).
async fn register(State(iam): State<IamState>, Json(body): Json<CredentialsRequest>) -> Response {
    match iam.register.execute(&body.email, &body.password).await {
        Ok(user) => (StatusCode::CREATED, Json(PrincipalResponse::from(user))).into_response(),
        Err(error) => error_response(&error),
    }
}

/// `POST /auth/login` — exchange credentials for a JWT.
async fn login(State(iam): State<IamState>, Json(body): Json<CredentialsRequest>) -> Response {
    match iam.login.execute(&body.email, &body.password).await {
        Ok(issued) => Json(AuthTokenResponse::from(issued)).into_response(),
        Err(error) => error_response(&error),
    }
}

/// `GET /auth/me` — the current principal (requires auth).
async fn me(
    State(iam): State<IamState>,
    AuthenticatedUser(principal): AuthenticatedUser,
) -> Response {
    match iam.current_user.execute(principal.user_id).await {
        Ok(user) => Json(PrincipalResponse::from(user)).into_response(),
        Err(error) => error_response(&error),
    }
}

/// `POST /users/{id}/roles` — assign a role (requires admin). The extractor
/// guarantees authentication (401); the use case enforces the admin role (403).
async fn assign_role(
    State(iam): State<IamState>,
    AuthenticatedUser(principal): AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<AssignRoleRequest>,
) -> Response {
    match iam.assign_role.execute(&principal, id, body.role).await {
        Ok(user) => Json(PrincipalResponse::from(user)).into_response(),
        Err(error) => error_response(&error),
    }
}

/// Map a domain error to its HTTP response. 401 (unauthenticated) and 403
/// (authenticated but unauthorized) are kept distinct.
fn error_response(error: &IamError) -> Response {
    let (status, code, message) = match error {
        IamError::EmailAlreadyExists => (
            StatusCode::CONFLICT,
            "email_taken",
            "email already registered",
        ),
        IamError::InvalidCredentials => (
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            "invalid email or password",
        ),
        IamError::Unauthorized => (
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "authentication required",
        ),
        IamError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "insufficient role"),
        IamError::UserNotFound => (StatusCode::NOT_FOUND, "not_found", "user not found"),
        IamError::WeakPassword => (
            StatusCode::BAD_REQUEST,
            "weak_password",
            "password must be at least 8 characters",
        ),
        IamError::InvalidEmail => (StatusCode::BAD_REQUEST, "invalid_email", "invalid email"),
        // Internal failures: generic 500, no detail leaked.
        IamError::Hashing(_) | IamError::Token(_) | IamError::Repository(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "internal error",
        ),
    };
    (status, Json(ErrorBody::new(code, message))).into_response()
}

/// Mount the IAM routes with the use cases as state.
pub fn router(state: IamState) -> Router {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/me", get(me))
        .route("/users/{id}/roles", post(assign_role))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{PasswordHasher, Role, User};
    use crate::infrastructure::argon2_hasher::Argon2PasswordHasher;
    use crate::infrastructure::in_memory_users::InMemoryUserRepository;
    use crate::infrastructure::jwt::JwtTokenService;
    use axum::body::Body;
    use axum::http::header::CONTENT_TYPE;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    const ADMIN_EMAIL: &str = "admin@library.local";
    const ADMIN_PASSWORD: &str = "admin-password-123";

    fn build_app() -> Router {
        let hasher = Arc::new(Argon2PasswordHasher::new());
        let tokens: Arc<dyn TokenService> =
            Arc::new(JwtTokenService::new(b"test-secret-not-real", 3600));

        let admin = User::new(
            Uuid::new_v4(),
            ADMIN_EMAIL.to_owned(),
            hasher.hash(ADMIN_PASSWORD).expect("hash admin"),
            Role::Admin,
        );
        let users = Arc::new(InMemoryUserRepository::seeded_with(vec![admin]));

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
        router(state)
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
                .expect("request builds"),
            None => builder.body(Body::empty()).expect("request builds"),
        };

        let response = app.clone().oneshot(request).await.expect("router responds");
        let status = response.status();
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("body collects")
            .to_bytes();
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).expect("valid json")
        };
        (status, value)
    }

    async fn login(app: &Router, email: &str, password: &str) -> (StatusCode, Value) {
        call(
            app,
            "POST",
            "/auth/login",
            None,
            Some(json!({"email": email, "password": password})),
        )
        .await
    }

    #[tokio::test]
    async fn register_creates_a_member() {
        let app = build_app();
        let (status, body) = call(
            &app,
            "POST",
            "/auth/register",
            None,
            Some(json!({"email": "newbie@example.com", "password": "longenough"})),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["role"], "member");
        assert_eq!(body["email"], "newbie@example.com");
        assert!(body.get("password_hash").is_none());
        assert!(body.get("password").is_none());
    }

    #[tokio::test]
    async fn login_succeeds_and_fails_correctly() {
        let app = build_app();
        call(
            &app,
            "POST",
            "/auth/register",
            None,
            Some(json!({"email": "u@example.com", "password": "longenough"})),
        )
        .await;

        let (ok_status, ok_body) = login(&app, "u@example.com", "longenough").await;
        assert_eq!(ok_status, StatusCode::OK);
        assert_eq!(ok_body["token_type"], "Bearer");
        assert!(ok_body["token"].as_str().expect("token").contains('.'));

        let (bad_status, bad_body) = login(&app, "u@example.com", "wrong-password").await;
        assert_eq!(bad_status, StatusCode::UNAUTHORIZED);
        assert_eq!(bad_body["code"], "invalid_credentials");

        // Unknown email is indistinguishable from a wrong password.
        let (missing_status, missing_body) = login(&app, "ghost@example.com", "whatever1").await;
        assert_eq!(missing_status, StatusCode::UNAUTHORIZED);
        assert_eq!(missing_body["code"], "invalid_credentials");
    }

    #[tokio::test]
    async fn me_requires_a_valid_token() {
        let app = build_app();
        call(
            &app,
            "POST",
            "/auth/register",
            None,
            Some(json!({"email": "me@example.com", "password": "longenough"})),
        )
        .await;
        let (_, token_body) = login(&app, "me@example.com", "longenough").await;
        let token = token_body["token"].as_str().expect("token");

        let (ok_status, ok_body) = call(&app, "GET", "/auth/me", Some(token), None).await;
        assert_eq!(ok_status, StatusCode::OK);
        assert_eq!(ok_body["email"], "me@example.com");
        assert_eq!(ok_body["role"], "member");

        let (no_auth, no_body) = call(&app, "GET", "/auth/me", None, None).await;
        assert_eq!(no_auth, StatusCode::UNAUTHORIZED);
        assert_eq!(no_body["code"], "unauthorized");

        let (bad_auth, _) = call(&app, "GET", "/auth/me", Some("garbage.token.here"), None).await;
        assert_eq!(bad_auth, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn role_assignment_is_admin_only() {
        let app = build_app();

        // A fresh member.
        let (_, member) = call(
            &app,
            "POST",
            "/auth/register",
            None,
            Some(json!({"email": "target@example.com", "password": "longenough"})),
        )
        .await;
        let member_id = member["id"].as_str().expect("id").to_owned();

        let (_, member_token_body) = login(&app, "target@example.com", "longenough").await;
        let member_token = member_token_body["token"].as_str().expect("token");

        let uri = format!("/users/{member_id}/roles");

        // No token at all → 401.
        let (unauth, _) = call(&app, "POST", &uri, None, Some(json!({"role": "librarian"}))).await;
        assert_eq!(unauth, StatusCode::UNAUTHORIZED);

        // Authenticated member (not admin) → 403.
        let (forbidden, forbidden_body) = call(
            &app,
            "POST",
            &uri,
            Some(member_token),
            Some(json!({"role": "librarian"})),
        )
        .await;
        assert_eq!(forbidden, StatusCode::FORBIDDEN);
        assert_eq!(forbidden_body["code"], "forbidden");

        // Admin → 200 and the role changes.
        let (_, admin_token_body) = login(&app, ADMIN_EMAIL, ADMIN_PASSWORD).await;
        let admin_token = admin_token_body["token"].as_str().expect("token");

        let (ok, ok_body) = call(
            &app,
            "POST",
            &uri,
            Some(admin_token),
            Some(json!({"role": "librarian"})),
        )
        .await;
        assert_eq!(ok, StatusCode::OK);
        assert_eq!(ok_body["role"], "librarian");
        assert_eq!(ok_body["id"], member_id);
    }
}
