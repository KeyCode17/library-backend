use std::sync::Arc;

use axum::extract::{FromRef, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use iam::domain::TokenService;
use iam::presentation::AuthenticatedUser;

use super::dto::{BorrowRequest, ErrorBody, LoanDto, LoanListResponse};
use crate::application::{ApproveLoan, BorrowBook, ListLoans, ReturnLoan};
use crate::domain::pagination::DEFAULT_PAGE_SIZE;
use crate::domain::{LendingError, PageRequest};

/// State injected into the lending routes: one use case per operation plus the
/// token service the bearer extractor needs. Cheap to clone (just `Arc`s).
#[derive(Clone)]
pub struct LendingState {
    pub borrow: Arc<BorrowBook>,
    pub return_loan: Arc<ReturnLoan>,
    pub approve: Arc<ApproveLoan>,
    pub list: Arc<ListLoans>,
    pub tokens: Arc<dyn TokenService>,
}

/// Lets IAM's `AuthenticatedUser` extractor pull the token service from this
/// context's state — reuse without depending on the whole `IamState`.
impl FromRef<LendingState> for Arc<dyn TokenService> {
    fn from_ref(state: &LendingState) -> Self {
        state.tokens.clone()
    }
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    page: Option<u32>,
    page_size: Option<u32>,
}

/// `POST /loans` — borrow a book.
async fn borrow(
    State(state): State<LendingState>,
    AuthenticatedUser(principal): AuthenticatedUser,
    Json(body): Json<BorrowRequest>,
) -> Response {
    match state.borrow.execute(&principal, body.book_id).await {
        Ok(loan) => (StatusCode::CREATED, Json(LoanDto::from(loan))).into_response(),
        Err(error) => error_response(&error),
    }
}

/// `POST /loans/{id}/return` — return a borrowed book (owner or staff).
async fn return_loan(
    State(state): State<LendingState>,
    AuthenticatedUser(principal): AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Response {
    match state.return_loan.execute(&principal, id).await {
        Ok(loan) => Json(LoanDto::from(loan)).into_response(),
        Err(error) => error_response(&error),
    }
}

/// `POST /loans/{id}/approve` — staff approves a returned loan.
async fn approve(
    State(state): State<LendingState>,
    AuthenticatedUser(principal): AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Response {
    match state.approve.execute(&principal, id).await {
        Ok(loan) => Json(LoanDto::from(loan)).into_response(),
        Err(error) => error_response(&error),
    }
}

/// `GET /loans` — member sees own loans; staff sees all.
async fn list(
    State(state): State<LendingState>,
    AuthenticatedUser(principal): AuthenticatedUser,
    Query(query): Query<ListQuery>,
) -> Response {
    let request = PageRequest::new(
        query.page.unwrap_or(1),
        query.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
    );
    match state.list.execute(&principal, request).await {
        Ok(page) => Json(LoanListResponse::from(page)).into_response(),
        Err(error) => error_response(&error),
    }
}

fn error_response(error: &LendingError) -> Response {
    let (status, code, message) = match error {
        LendingError::BookNotFound => (StatusCode::NOT_FOUND, "book_not_found", "book not found"),
        LendingError::BookUnavailable => (
            StatusCode::CONFLICT,
            "book_unavailable",
            "book is not available to borrow",
        ),
        LendingError::LoanNotFound => (StatusCode::NOT_FOUND, "not_found", "loan not found"),
        LendingError::Forbidden => (
            StatusCode::FORBIDDEN,
            "forbidden",
            "not permitted to act on this loan",
        ),
        LendingError::InvalidState => (
            StatusCode::CONFLICT,
            "invalid_state",
            "loan is not in a state that allows this action",
        ),
        LendingError::Dependency(_) | LendingError::Repository(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "internal error",
        ),
    };
    (status, Json(ErrorBody::new(code, message))).into_response()
}

/// Mount the lending routes (all require a bearer token) with the use cases as
/// state.
pub fn router(state: LendingState) -> Router {
    Router::new()
        .route("/loans", post(borrow).get(list))
        .route("/loans/{id}/return", post(return_loan))
        .route("/loans/{id}/approve", post(approve))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{BookGateway, ClaimOutcome, Clock};
    use crate::infrastructure::in_memory_loans::InMemoryLoanRepository;
    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
    use axum::http::Request;
    use chrono::{DateTime, Utc};
    use http_body_util::BodyExt;
    use iam::domain::{AuthPrincipal, Role};
    use iam::infrastructure::jwt::JwtTokenService;
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::sync::RwLock;
    use tower::ServiceExt;

    const BOOK: Uuid = uuid::uuid!("00000000-0000-4000-8000-0000000000aa");
    const MISSING_BOOK: Uuid = uuid::uuid!("00000000-0000-4000-8000-0000000000bb");

    /// Fake catalog gateway: an in-memory `book_id -> available` map.
    struct FakeBookGateway {
        books: RwLock<HashMap<Uuid, bool>>,
    }

    impl FakeBookGateway {
        fn with(book: Uuid, available: bool) -> Self {
            let mut map = HashMap::new();
            map.insert(book, available);
            Self {
                books: RwLock::new(map),
            }
        }

        fn is_available_now(&self, book: Uuid) -> Option<bool> {
            self.books.read().unwrap().get(&book).copied()
        }
    }

    #[async_trait]
    impl BookGateway for FakeBookGateway {
        async fn claim_for_borrow(&self, book_id: Uuid) -> Result<ClaimOutcome, LendingError> {
            let mut guard = self.books.write().unwrap();
            match guard.get_mut(&book_id) {
                None => Ok(ClaimOutcome::NotFound),
                Some(slot) if !*slot => Ok(ClaimOutcome::Unavailable),
                Some(slot) => {
                    *slot = false;
                    Ok(ClaimOutcome::Claimed)
                }
            }
        }
        async fn set_available(&self, book_id: Uuid, available: bool) -> Result<(), LendingError> {
            match self.books.write().unwrap().get_mut(&book_id) {
                Some(slot) => {
                    *slot = available;
                    Ok(())
                }
                None => Err(LendingError::BookNotFound),
            }
        }
    }

    struct FixedClock(DateTime<Utc>);
    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    struct Harness {
        app: Router,
        tokens: Arc<dyn TokenService>,
        gateway: Arc<FakeBookGateway>,
        member: Uuid,
        other_member: Uuid,
        staff: Uuid,
    }

    fn build() -> Harness {
        let tokens: Arc<dyn TokenService> =
            Arc::new(JwtTokenService::new(b"lending-test-secret", 3600));
        let loans = Arc::new(InMemoryLoanRepository::new());
        let gateway = Arc::new(FakeBookGateway::with(BOOK, true));
        let books: Arc<dyn BookGateway> = gateway.clone();
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(
            DateTime::from_timestamp(1_700_000_000, 0).expect("valid timestamp"),
        ));

        let state = LendingState {
            borrow: Arc::new(BorrowBook::new(loans.clone(), books.clone(), clock.clone())),
            return_loan: Arc::new(ReturnLoan::new(loans.clone(), books.clone(), clock.clone())),
            approve: Arc::new(ApproveLoan::new(loans.clone(), clock)),
            list: Arc::new(ListLoans::new(loans)),
            tokens: tokens.clone(),
        };

        Harness {
            app: router(state),
            tokens,
            gateway,
            member: Uuid::new_v4(),
            other_member: Uuid::new_v4(),
            staff: Uuid::new_v4(),
        }
    }

    impl Harness {
        fn token(&self, user_id: Uuid, role: Role) -> String {
            self.tokens
                .issue(&AuthPrincipal { user_id, role })
                .expect("issue token")
                .token
        }

        async fn call(
            &self,
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

            let response = self.app.clone().oneshot(request).await.expect("responds");
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
    }

    #[tokio::test]
    async fn borrow_requires_auth() {
        let h = build();
        let (status, _) = h
            .call("POST", "/loans", None, Some(json!({"book_id": BOOK})))
            .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn borrow_succeeds_then_book_is_unavailable() {
        let h = build();
        let token = h.token(h.member, Role::Member);

        let (status, loan) = h
            .call(
                "POST",
                "/loans",
                Some(&token),
                Some(json!({"book_id": BOOK})),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(loan["status"], "borrowed");
        assert_eq!(loan["book_id"], BOOK.to_string());
        assert!(loan["due_at"].is_string());
        assert_eq!(h.gateway.is_available_now(BOOK), Some(false));

        // A second borrow of the same book is a conflict.
        let (again, body) = h
            .call(
                "POST",
                "/loans",
                Some(&token),
                Some(json!({"book_id": BOOK})),
            )
            .await;
        assert_eq!(again, StatusCode::CONFLICT);
        assert_eq!(body["code"], "book_unavailable");
    }

    #[tokio::test]
    async fn borrow_missing_book_is_404() {
        let h = build();
        let token = h.token(h.member, Role::Member);
        let (status, body) = h
            .call(
                "POST",
                "/loans",
                Some(&token),
                Some(json!({"book_id": MISSING_BOOK})),
            )
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["code"], "book_not_found");
    }

    #[tokio::test]
    async fn owner_returns_and_book_becomes_available_but_other_member_cannot() {
        let h = build();
        let member_token = h.token(h.member, Role::Member);
        let other_token = h.token(h.other_member, Role::Member);

        let (_, loan) = h
            .call(
                "POST",
                "/loans",
                Some(&member_token),
                Some(json!({"book_id": BOOK})),
            )
            .await;
        let loan_id = loan["id"].as_str().expect("id").to_owned();

        // A different member must not return someone else's loan.
        let (forbidden, body) = h
            .call(
                "POST",
                &format!("/loans/{loan_id}/return"),
                Some(&other_token),
                None,
            )
            .await;
        assert_eq!(forbidden, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "forbidden");

        // The owner returns successfully; the book is available again.
        let (ok, returned) = h
            .call(
                "POST",
                &format!("/loans/{loan_id}/return"),
                Some(&member_token),
                None,
            )
            .await;
        assert_eq!(ok, StatusCode::OK);
        assert_eq!(returned["status"], "returned");
        assert_eq!(h.gateway.is_available_now(BOOK), Some(true));
    }

    #[tokio::test]
    async fn approve_is_staff_only() {
        let h = build();
        let member_token = h.token(h.member, Role::Member);
        let staff_token = h.token(h.staff, Role::Librarian);

        let (_, loan) = h
            .call(
                "POST",
                "/loans",
                Some(&member_token),
                Some(json!({"book_id": BOOK})),
            )
            .await;
        let loan_id = loan["id"].as_str().expect("id").to_owned();
        h.call(
            "POST",
            &format!("/loans/{loan_id}/return"),
            Some(&member_token),
            None,
        )
        .await;

        // A member cannot approve.
        let (forbidden, body) = h
            .call(
                "POST",
                &format!("/loans/{loan_id}/approve"),
                Some(&member_token),
                None,
            )
            .await;
        assert_eq!(forbidden, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "forbidden");

        // Staff can.
        let (ok, approved) = h
            .call(
                "POST",
                &format!("/loans/{loan_id}/approve"),
                Some(&staff_token),
                None,
            )
            .await;
        assert_eq!(ok, StatusCode::OK);
        assert_eq!(approved["status"], "approved");
    }

    #[tokio::test]
    async fn members_only_see_their_own_loans() {
        let h = build();
        let member_token = h.token(h.member, Role::Member);
        let staff_token = h.token(h.staff, Role::Librarian);

        // Member borrows BOOK; staff borrows another book (seed it available).
        h.gateway.books.write().unwrap().insert(MISSING_BOOK, true);
        h.call(
            "POST",
            "/loans",
            Some(&member_token),
            Some(json!({"book_id": BOOK})),
        )
        .await;
        h.call(
            "POST",
            "/loans",
            Some(&staff_token),
            Some(json!({"book_id": MISSING_BOOK})),
        )
        .await;

        // Member sees only their own loan.
        let (status, mine) = h.call("GET", "/loans", Some(&member_token), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(mine["pagination"]["total"], 1);
        assert_eq!(mine["data"][0]["user_id"], h.member.to_string());

        // Staff sees all loans.
        let (_, all) = h.call("GET", "/loans", Some(&staff_token), None).await;
        assert_eq!(all["pagination"]["total"], 2);
    }
}
