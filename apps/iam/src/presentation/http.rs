use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts, Path, Query, State};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::dto::{
    AssignRoleRequest, AuthTokenResponse, ChangePasswordRequest, CreateUserRequest,
    CredentialsRequest, ErrorBody, ForgotPasswordRequest, PrincipalResponse, ResetPasswordRequest,
    UpdateMeRequest, UpdateUserRequest, UserListResponse, UserSummary, VerifyEmailRequest,
};
use crate::application::{
    AssignRole, ChangePassword, CreateUser, DeleteMe, DeleteUser, ForgotPassword, GetCurrentUser,
    ListUsers, LoginUser, RegisterUser, ResetPassword, UpdateMe, UpdateUser, VerifyEmail,
};
use crate::domain::pagination::DEFAULT_PAGE_SIZE;
use crate::domain::{AuthPrincipal, IamError, PageRequest, TokenService};

/// State injected into the IAM routes: one use case per operation plus the token
/// service (used by the bearer extractor). Cheap to clone (just `Arc`s).
#[derive(Clone)]
pub struct IamState {
    pub register: Arc<RegisterUser>,
    pub login: Arc<LoginUser>,
    pub current_user: Arc<GetCurrentUser>,
    pub update_me: Arc<UpdateMe>,
    pub delete_me: Arc<DeleteMe>,
    pub change_password: Arc<ChangePassword>,
    pub verify_email: Arc<VerifyEmail>,
    pub forgot_password: Arc<ForgotPassword>,
    pub reset_password: Arc<ResetPassword>,
    pub list_users: Arc<ListUsers>,
    pub create_user: Arc<CreateUser>,
    pub update_user: Arc<UpdateUser>,
    pub delete_user: Arc<DeleteUser>,
    pub assign_role: Arc<AssignRole>,
    pub tokens: Arc<dyn TokenService>,
}

impl FromRef<IamState> for Arc<dyn TokenService> {
    fn from_ref(state: &IamState) -> Self {
        state.tokens.clone()
    }
}

/// Extractor proving a valid bearer token. Depends only on `Arc<dyn TokenService>`
/// (via `FromRef`), so other contexts reuse it.
pub struct AuthenticatedUser(pub AuthPrincipal);

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
    Arc<dyn TokenService>: FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let tokens = Arc::<dyn TokenService>::from_ref(state);
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .ok_or_else(unauthorized)?;
        let principal = tokens.verify(token).map_err(|_| unauthorized())?;
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

#[derive(Debug, Deserialize)]
struct ListQuery {
    page: Option<u32>,
    page_size: Option<u32>,
}

fn page_request(page: Option<u32>, page_size: Option<u32>) -> PageRequest {
    PageRequest::new(page.unwrap_or(1), page_size.unwrap_or(DEFAULT_PAGE_SIZE))
}

// ---- auth -----------------------------------------------------------------

async fn register(State(iam): State<IamState>, Json(body): Json<CredentialsRequest>) -> Response {
    match iam.register.execute(&body.email, &body.password).await {
        Ok(user) => (StatusCode::CREATED, Json(PrincipalResponse::from(user))).into_response(),
        Err(error) => error_response(&error),
    }
}

async fn login(State(iam): State<IamState>, Json(body): Json<CredentialsRequest>) -> Response {
    match iam.login.execute(&body.email, &body.password).await {
        Ok(issued) => Json(AuthTokenResponse::from(issued)).into_response(),
        Err(error) => error_response(&error),
    }
}

async fn me(
    State(iam): State<IamState>,
    AuthenticatedUser(principal): AuthenticatedUser,
) -> Response {
    match iam.current_user.execute(principal.user_id).await {
        Ok(user) => Json(PrincipalResponse::from(user)).into_response(),
        Err(error) => error_response(&error),
    }
}

async fn update_me(
    State(iam): State<IamState>,
    AuthenticatedUser(principal): AuthenticatedUser,
    Json(body): Json<UpdateMeRequest>,
) -> Response {
    match iam.update_me.execute(&principal, &body.email).await {
        Ok(user) => Json(PrincipalResponse::from(user)).into_response(),
        Err(error) => error_response(&error),
    }
}

async fn delete_me(
    State(iam): State<IamState>,
    AuthenticatedUser(principal): AuthenticatedUser,
) -> Response {
    match iam.delete_me.execute(&principal).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => error_response(&error),
    }
}

async fn change_password(
    State(iam): State<IamState>,
    AuthenticatedUser(principal): AuthenticatedUser,
    Json(body): Json<ChangePasswordRequest>,
) -> Response {
    match iam
        .change_password
        .execute(&principal, &body.current_password, &body.new_password)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => error_response(&error),
    }
}

async fn verify_email(
    State(iam): State<IamState>,
    Json(body): Json<VerifyEmailRequest>,
) -> Response {
    match iam.verify_email.execute(&body.token).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => error_response(&error),
    }
}

async fn forgot_password(
    State(iam): State<IamState>,
    Json(body): Json<ForgotPasswordRequest>,
) -> Response {
    // Always 202 — never reveal whether the email exists.
    let _ = iam.forgot_password.execute(&body.email).await;
    StatusCode::ACCEPTED.into_response()
}

async fn reset_password(
    State(iam): State<IamState>,
    Json(body): Json<ResetPasswordRequest>,
) -> Response {
    match iam
        .reset_password
        .execute(&body.token, &body.new_password)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => error_response(&error),
    }
}

// ---- admin user management ------------------------------------------------

async fn list_users(
    State(iam): State<IamState>,
    AuthenticatedUser(principal): AuthenticatedUser,
    Query(query): Query<ListQuery>,
) -> Response {
    match iam
        .list_users
        .execute(&principal, page_request(query.page, query.page_size))
        .await
    {
        Ok(page) => Json(UserListResponse::from(page)).into_response(),
        Err(error) => error_response(&error),
    }
}

async fn create_user(
    State(iam): State<IamState>,
    AuthenticatedUser(principal): AuthenticatedUser,
    Json(body): Json<CreateUserRequest>,
) -> Response {
    match iam
        .create_user
        .execute(&principal, &body.email, &body.password, body.role)
        .await
    {
        Ok(user) => (StatusCode::CREATED, Json(UserSummary::from(user))).into_response(),
        Err(error) => error_response(&error),
    }
}

async fn update_user(
    State(iam): State<IamState>,
    AuthenticatedUser(principal): AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateUserRequest>,
) -> Response {
    match iam
        .update_user
        .execute(&principal, id, body.email, body.active)
        .await
    {
        Ok(user) => Json(UserSummary::from(user)).into_response(),
        Err(error) => error_response(&error),
    }
}

async fn delete_user(
    State(iam): State<IamState>,
    AuthenticatedUser(principal): AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Response {
    match iam.delete_user.execute(&principal, id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => error_response(&error),
    }
}

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
        IamError::InvalidToken => (StatusCode::BAD_REQUEST, "invalid_token", "invalid token"),
        IamError::TokenExpired => (StatusCode::BAD_REQUEST, "token_expired", "token expired"),
        IamError::TokenConsumed => (StatusCode::BAD_REQUEST, "token_used", "token already used"),
        IamError::LastAdmin => (
            StatusCode::CONFLICT,
            "last_admin",
            "cannot remove the last admin",
        ),
        IamError::Hashing(_)
        | IamError::Token(_)
        | IamError::Email(_)
        | IamError::Repository(_) => (
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
        .route("/auth/me", get(me).patch(update_me).delete(delete_me))
        .route("/auth/change-password", post(change_password))
        .route("/auth/verify-email", post(verify_email))
        .route("/auth/forgot-password", post(forgot_password))
        .route("/auth/reset-password", post(reset_password))
        .route("/users", get(list_users).post(create_user))
        .route("/users/{id}", patch(update_user).delete(delete_user))
        .route("/users/{id}/roles", post(assign_role))
        .with_state(state)
}
