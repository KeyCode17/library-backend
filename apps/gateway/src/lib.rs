//! Gateway composition root.
//!
//! The gateway is the only binary context (per ADR 0002): it opens the Postgres
//! pool, runs migrations, seeds, constructs the SeaORM adapters, injects them into
//! each context's use cases, and merges the context routers. Persistence is
//! Postgres/SeaORM for every context (the in-memory adapters remain only for the
//! contexts' own DB-free unit tests).

pub mod presentation;

mod catalog_bridge;
mod loan_source_bridge;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;

use persistence::sea_orm::{DatabaseConnection, DbErr};

use catalog::application::{GetBook, ListBooks};
use catalog::domain::BookRepository;
use catalog::infrastructure::SeaOrmBookRepository;
use catalog::presentation::CatalogState;

use iam::application::{
    AssignRole, ChangePassword, CreateUser, DeleteMe, DeleteUser, ForgotPassword, GetCurrentUser,
    ListUsers, LoginUser, RegisterUser, ResetPassword, UpdateMe, UpdateUser, VerifyEmail,
};
use iam::domain::{
    Clock as IamClock, EmailSender, EmailTokenRepository, PasswordHasher, TokenGenerator,
    TokenService, UserRepository,
};
use iam::infrastructure::{
    Argon2PasswordHasher, IamConfig, JwtTokenService, RandomTokenGenerator,
    SeaOrmEmailTokenRepository, SeaOrmUserRepository, SystemClock as IamClockImpl,
};
use iam::presentation::IamState;

use lending::application::{ApproveLoan, BorrowBook, ListLoans, ReturnLoan};
use lending::domain::{BookGateway, Clock as LendingClock, LoanRepository};
use lending::infrastructure::{SeaOrmLoanRepository, SystemClock as LendingClockImpl};
use lending::presentation::LendingState;

use recommender::Recommender;

use chat::domain::{Clock as ChatClock, MessageBroadcaster, MessageRepository};
use chat::infrastructure::{RoomHub, SeaOrmMessageRepository, SystemClock as ChatClockImpl};

use notification::application::NotificationScheduler;
use notification::domain::Clock as NotificationClock;
use notification::infrastructure::{
    FcmPushSender, SeaOrmDeviceRepository, SeaOrmReminderRepository,
    SystemClock as NotificationClockImpl,
};

use catalog_bridge::CatalogBookGateway;
use loan_source_bridge::LendingLoanSource;
use migration::{Migrator, MigratorTrait};
use presentation::recommend::RecommendState;

/// How often the notification scheduler scans loan due dates.
const SCHEDULER_PERIOD_SECS: u64 = 3600;

/// Apply all pending migrations.
pub async fn migrate(db: &DatabaseConnection) -> Result<(), DbErr> {
    Migrator::up(db, None).await
}

/// Build the application and the notification scheduler over a Postgres pool.
///
/// Seeds the catalog and the admin user idempotently, then wires the SeaORM
/// adapters into every context.
pub async fn build(
    db: DatabaseConnection,
    email_sender: Arc<dyn EmailSender>,
) -> (Router, NotificationScheduler) {
    catalog::infrastructure::seed::seed_books_if_empty(&db)
        .await
        .expect("seed catalog");

    let iam_state = build_iam_state(db.clone(), email_sender).await;

    let books: Arc<dyn BookRepository> = Arc::new(SeaOrmBookRepository::new(db.clone()));
    let loans: Arc<dyn LoanRepository> = Arc::new(SeaOrmLoanRepository::new(db.clone()));

    let (notification_router, scheduler) =
        notification_setup(db.clone(), loans.clone(), iam_state.tokens.clone());

    let router = Router::new()
        .merge(presentation::health::routes())
        .merge(catalog_router(books.clone()))
        .merge(iam::presentation::router(iam_state.clone()))
        .merge(lending_router(
            books.clone(),
            loans,
            iam_state.tokens.clone(),
        ))
        .merge(recommend_router(books))
        .merge(chat_router(db, iam_state.tokens.clone()))
        .merge(notification_router);

    (router, scheduler)
}

/// Public, read-only catalog (no auth — deliberately).
fn catalog_router(books: Arc<dyn BookRepository>) -> Router {
    let state = CatalogState {
        list_books: Arc::new(ListBooks::new(books.clone())),
        get_book: Arc::new(GetBook::new(books)),
    };
    catalog::presentation::router(state)
}

/// IAM: auth + RBAC + user management + email flows, persisted in Postgres.
/// Secrets come from config/env; the admin is seeded idempotently.
async fn build_iam_state(db: DatabaseConnection, email_sender: Arc<dyn EmailSender>) -> IamState {
    let config = IamConfig::from_env();
    let hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2PasswordHasher::new());
    let tokens: Arc<dyn TokenService> = Arc::new(JwtTokenService::new(
        &config.jwt_secret,
        config.token_ttl_secs,
    ));
    let users: Arc<dyn UserRepository> = Arc::new(SeaOrmUserRepository::new(db.clone()));
    let email_tokens: Arc<dyn EmailTokenRepository> = Arc::new(SeaOrmEmailTokenRepository::new(db));
    let token_generator: Arc<dyn TokenGenerator> = Arc::new(RandomTokenGenerator::new());
    let clock: Arc<dyn IamClock> = Arc::new(IamClockImpl);
    let base_url = config.public_base_url.clone();

    seed_admin(users.as_ref(), hasher.as_ref(), &config).await;

    IamState {
        register: Arc::new(RegisterUser::new(
            users.clone(),
            hasher.clone(),
            clock.clone(),
            email_tokens.clone(),
            token_generator.clone(),
            email_sender.clone(),
            base_url.clone(),
        )),
        login: Arc::new(LoginUser::new(
            users.clone(),
            hasher.clone(),
            tokens.clone(),
        )),
        current_user: Arc::new(GetCurrentUser::new(users.clone())),
        update_me: Arc::new(UpdateMe::new(users.clone())),
        delete_me: Arc::new(DeleteMe::new(users.clone())),
        change_password: Arc::new(ChangePassword::new(users.clone(), hasher.clone())),
        verify_email: Arc::new(VerifyEmail::new(
            users.clone(),
            email_tokens.clone(),
            token_generator.clone(),
            clock.clone(),
        )),
        forgot_password: Arc::new(ForgotPassword::new(
            users.clone(),
            email_tokens.clone(),
            token_generator.clone(),
            email_sender,
            clock.clone(),
            base_url,
        )),
        reset_password: Arc::new(ResetPassword::new(
            users.clone(),
            hasher.clone(),
            email_tokens,
            token_generator,
            clock.clone(),
        )),
        list_users: Arc::new(ListUsers::new(users.clone())),
        create_user: Arc::new(CreateUser::new(users.clone(), hasher, clock)),
        update_user: Arc::new(UpdateUser::new(users.clone())),
        delete_user: Arc::new(DeleteUser::new(users.clone())),
        assign_role: Arc::new(AssignRole::new(users)),
        tokens,
    }
}

async fn seed_admin(users: &dyn UserRepository, hasher: &dyn PasswordHasher, config: &IamConfig) {
    let email = config.admin_email.to_lowercase();
    let exists = users
        .find_by_email(&email)
        .await
        .expect("check seed admin")
        .is_some();
    if exists {
        return;
    }
    let admin = config.seed_admin(hasher).expect("build admin user");
    match users.insert(admin).await {
        // A concurrent boot may have inserted it first — that's fine.
        Ok(()) | Err(iam::domain::IamError::EmailAlreadyExists) => {}
        Err(error) => panic!("seed admin failed: {error}"),
    }
}

/// Lending: the loan lifecycle. The loan store is shared with the notification
/// scheduler; book availability is bridged to catalog (atomic borrow claim).
fn lending_router(
    books: Arc<dyn BookRepository>,
    loans: Arc<dyn LoanRepository>,
    tokens: Arc<dyn TokenService>,
) -> Router {
    let clock: Arc<dyn LendingClock> = Arc::new(LendingClockImpl);
    let book_gateway: Arc<dyn BookGateway> = Arc::new(CatalogBookGateway::new(books));

    let state = LendingState {
        borrow: Arc::new(BorrowBook::new(
            loans.clone(),
            book_gateway.clone(),
            clock.clone(),
        )),
        return_loan: Arc::new(ReturnLoan::new(loans.clone(), book_gateway, clock.clone())),
        approve: Arc::new(ApproveLoan::new(loans.clone(), clock)),
        list: Arc::new(ListLoans::new(loans)),
        tokens,
    };
    lending::presentation::router(state)
}

/// Public recommendations via the pure `recommender` crate.
fn recommend_router(books: Arc<dyn BookRepository>) -> Router {
    presentation::recommend::routes(RecommendState {
        recommender: Arc::new(Recommender::new()),
        books,
    })
}

/// Chat: WebSocket + REST history. History is persisted; the room hub is the live
/// (in-memory) broadcast registry.
fn chat_router(db: DatabaseConnection, tokens: Arc<dyn TokenService>) -> Router {
    let messages: Arc<dyn MessageRepository> = Arc::new(SeaOrmMessageRepository::new(db));
    let hub = Arc::new(RoomHub::new());
    let broadcaster: Arc<dyn MessageBroadcaster> = hub.clone();
    let clock: Arc<dyn ChatClock> = Arc::new(ChatClockImpl);

    let state = chat::presentation::ChatState {
        post_message: Arc::new(chat::application::PostMessage::new(
            messages.clone(),
            broadcaster,
            clock,
        )),
        history: Arc::new(chat::application::ListHistory::new(messages)),
        hub,
        tokens,
    };
    chat::presentation::router(state)
}

/// Notification: device registry + reminder history (REST) plus the background
/// due-date scheduler. FCM is credential-gated; the scheduler reads active loans
/// through the lending bridge.
fn notification_setup(
    db: DatabaseConnection,
    loans: Arc<dyn LoanRepository>,
    tokens: Arc<dyn TokenService>,
) -> (Router, NotificationScheduler) {
    use notification::application::{ListNotifications, RegisterDevice, RunReminderScan};
    use notification::domain::{DeviceRepository, PushSender, ReminderRepository};
    use notification::presentation::NotificationState;

    let devices: Arc<dyn DeviceRepository> = Arc::new(SeaOrmDeviceRepository::new(db.clone()));
    let reminders: Arc<dyn ReminderRepository> = Arc::new(SeaOrmReminderRepository::new(db));
    let push: Arc<dyn PushSender> = Arc::new(FcmPushSender::from_env());
    let clock: Arc<dyn NotificationClock> = Arc::new(NotificationClockImpl);
    let loan_source = Arc::new(LendingLoanSource::new(loans));

    let scan = Arc::new(RunReminderScan::new(
        loan_source,
        reminders.clone(),
        devices.clone(),
        push,
    ));

    let state = NotificationState {
        register_device: Arc::new(RegisterDevice::new(devices, clock.clone())),
        list_notifications: Arc::new(ListNotifications::new(reminders)),
        tokens,
    };

    let scheduler =
        NotificationScheduler::new(scan, clock, Duration::from_secs(SCHEDULER_PERIOD_SECS));
    (notification::presentation::router(state), scheduler)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use std::sync::Once;
    use testcontainers_modules::postgres::Postgres as PgImage;
    use testcontainers_modules::testcontainers::runners::AsyncRunner;
    use testcontainers_modules::testcontainers::ContainerAsync;
    use tower::ServiceExt;

    use iam::infrastructure::FakeEmailSender;

    const CLEAN_CODE_ID: &str = "00000000-0000-4000-8000-000000000002";
    const CLEAN_CODE_ISBN: &str = "978-0132350884";
    const ADMIN_EMAIL: &str = "admin@library.local";
    const ADMIN_PASSWORD: &str = "admin-pass-123";

    // Pin known IAM secrets once for the whole test process so admin login works
    // and tokens are stable. (edition 2021: set_var is a safe API.)
    static INIT_ENV: Once = Once::new();
    fn init_env() {
        INIT_ENV.call_once(|| {
            std::env::set_var("IAM_JWT_SECRET", "gateway-test-jwt-secret");
            std::env::set_var("IAM_ADMIN_EMAIL", ADMIN_EMAIL);
            std::env::set_var("IAM_ADMIN_PASSWORD", ADMIN_PASSWORD);
        });
    }

    /// Ephemeral Postgres + a fully composed app, with a fake email sender whose
    /// captured messages tests can read. Holds the container alive.
    struct TestApp {
        app: Router,
        email: Arc<FakeEmailSender>,
        _container: ContainerAsync<PgImage>,
    }

    async fn spawn() -> TestApp {
        init_env();
        let container = PgImage::default().start().await.expect("start postgres");
        let port = container.get_host_port_ipv4(5432).await.expect("host port");
        let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
        let db = persistence::db::connect(&url).await.expect("connect");
        migrate(&db).await.expect("migrate");
        let email = Arc::new(FakeEmailSender::new());
        let (app, _scheduler) = build(db, email.clone()).await;
        TestApp {
            app,
            email,
            _container: container,
        }
    }

    async fn admin_token(app: &Router) -> String {
        let (_, body) = call(
            app,
            "POST",
            "/auth/login",
            None,
            Some(json!({"email": ADMIN_EMAIL, "password": ADMIN_PASSWORD})),
        )
        .await;
        body["token"].as_str().expect("admin token").to_owned()
    }

    /// Extract the raw token from a captured verification/reset link.
    fn token_in_link(link: &str) -> String {
        link.split("token=")
            .nth(1)
            .expect("token in link")
            .to_owned()
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

    async fn member_token(app: &Router, email: &str) -> String {
        call(
            app,
            "POST",
            "/auth/register",
            None,
            Some(json!({"email": email, "password": "password123"})),
        )
        .await;
        let (_, body) = call(
            app,
            "POST",
            "/auth/login",
            None,
            Some(json!({"email": email, "password": "password123"})),
        )
        .await;
        body["token"].as_str().expect("token").to_owned()
    }

    #[tokio::test]
    async fn catalog_public_other_contexts_protected() {
        let h = spawn().await;
        assert_eq!(
            call(&h.app, "GET", "/books", None, None).await.0,
            StatusCode::OK
        );
        assert_eq!(
            call(&h.app, "GET", "/auth/me", None, None).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            call(&h.app, "GET", "/loans", None, None).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            call(&h.app, "GET", "/notifications", None, None).await.0,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn isbn_filter_resolves_one_book() {
        let h = spawn().await;
        let (status, body) = call(
            &h.app,
            "GET",
            &format!("/books?isbn={CLEAN_CODE_ISBN}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["pagination"]["total"], 1);
        assert_eq!(body["data"][0]["id"], CLEAN_CODE_ID);
    }

    #[tokio::test]
    async fn borrow_persists_and_flips_availability() {
        let h = spawn().await;
        let token = member_token(&h.app, "reader@example.com").await;

        let (before, before_body) = call(
            &h.app,
            "GET",
            &format!("/books/{CLEAN_CODE_ID}"),
            None,
            None,
        )
        .await;
        assert_eq!(before, StatusCode::OK);
        assert_eq!(before_body["available"], true);

        let (status, _) = call(
            &h.app,
            "POST",
            "/loans",
            Some(&token),
            Some(json!({"book_id": CLEAN_CODE_ID})),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        let (_, after_body) = call(
            &h.app,
            "GET",
            &format!("/books/{CLEAN_CODE_ID}"),
            None,
            None,
        )
        .await;
        assert_eq!(after_body["available"], false);
    }

    #[tokio::test]
    async fn concurrent_borrows_yield_exactly_one_active_loan() {
        let h = spawn().await;
        let token_a = member_token(&h.app, "a@example.com").await;
        let token_b = member_token(&h.app, "b@example.com").await;

        let app_a = h.app.clone();
        let app_b = h.app.clone();
        let body = json!({"book_id": CLEAN_CODE_ID});
        let (body_a, body_b) = (body.clone(), body);

        // Fire both borrows of the same book concurrently.
        let task_a = tokio::spawn(async move {
            call(&app_a, "POST", "/loans", Some(&token_a), Some(body_a))
                .await
                .0
        });
        let task_b = tokio::spawn(async move {
            call(&app_b, "POST", "/loans", Some(&token_b), Some(body_b))
                .await
                .0
        });

        let status_a = task_a.await.expect("task a");
        let status_b = task_b.await.expect("task b");

        let mut statuses = [status_a, status_b];
        statuses.sort_by_key(|s| s.as_u16());
        // Exactly one created, exactly one conflict — never two active loans.
        assert_eq!(statuses, [StatusCode::CREATED, StatusCode::CONFLICT]);
    }

    #[tokio::test]
    async fn recommend_ranks_the_seeded_catalog() {
        let h = spawn().await;
        let (status, body) = call(
            &h.app,
            "POST",
            "/recommend",
            None,
            Some(json!({"preferences": {"preferred_authors": ["Robert C. Martin"]}})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ranked"].as_array().expect("ranked").len(), 8);
        assert_eq!(body["ranked"][0], CLEAN_CODE_ID);
    }

    // ---- T-012: IAM v2 --------------------------------------------------------

    #[tokio::test]
    async fn user_management_is_admin_only_and_leaks_no_hash() {
        let h = spawn().await;
        let member = member_token(&h.app, "member@example.com").await;
        let admin = admin_token(&h.app).await;

        // 401 anon, 403 member, 200 admin.
        assert_eq!(
            call(&h.app, "GET", "/users", None, None).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            call(&h.app, "GET", "/users", Some(&member), None).await.0,
            StatusCode::FORBIDDEN
        );
        let (ok, list) = call(&h.app, "GET", "/users", Some(&admin), None).await;
        assert_eq!(ok, StatusCode::OK);
        assert!(list["pagination"]["total"].as_u64().expect("total") >= 2);
        // No password hash leaks anywhere in the listing.
        assert!(!list.to_string().contains("password_hash"));
        assert!(!list.to_string().contains("$argon2"));

        // Admin creates a user; member cannot.
        let (created, created_body) = call(
            &h.app,
            "POST",
            "/users",
            Some(&admin),
            Some(json!({"email": "made@example.com", "password": "password123", "role": "librarian"})),
        )
        .await;
        assert_eq!(created, StatusCode::CREATED);
        assert_eq!(created_body["role"], "librarian");
        let made_id = created_body["id"].as_str().expect("id").to_owned();

        assert_eq!(
            call(
                &h.app,
                "POST",
                "/users",
                Some(&member),
                Some(
                    json!({"email": "x@example.com", "password": "password123", "role": "member"})
                ),
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );

        // Admin patches + deletes the created user.
        let (patched, patched_body) = call(
            &h.app,
            "PATCH",
            &format!("/users/{made_id}"),
            Some(&admin),
            Some(json!({"active": false})),
        )
        .await;
        assert_eq!(patched, StatusCode::OK);
        assert_eq!(patched_body["active"], false);
        assert_eq!(
            call(
                &h.app,
                "DELETE",
                &format!("/users/{made_id}"),
                Some(&admin),
                None
            )
            .await
            .0,
            StatusCode::NO_CONTENT
        );
    }

    #[tokio::test]
    async fn last_admin_cannot_delete_or_demote_self() {
        let h = spawn().await;
        let admin = admin_token(&h.app).await;
        // The seeded admin is the only admin → self-delete is refused.
        let (status, body) = call(&h.app, "DELETE", "/auth/me", Some(&admin), None).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["code"], "last_admin");
    }

    #[tokio::test]
    async fn change_password_requires_correct_current() {
        let h = spawn().await;
        let token = member_token(&h.app, "cp@example.com").await;

        // Wrong current → 401.
        assert_eq!(
            call(
                &h.app,
                "POST",
                "/auth/change-password",
                Some(&token),
                Some(json!({"current_password": "wrongpass", "new_password": "newpassword1"})),
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED
        );

        // Correct current → 204, and the new password logs in.
        assert_eq!(
            call(
                &h.app,
                "POST",
                "/auth/change-password",
                Some(&token),
                Some(json!({"current_password": "password123", "new_password": "newpassword1"})),
            )
            .await
            .0,
            StatusCode::NO_CONTENT
        );
        let (relogin, _) = call(
            &h.app,
            "POST",
            "/auth/login",
            None,
            Some(json!({"email": "cp@example.com", "password": "newpassword1"})),
        )
        .await;
        assert_eq!(relogin, StatusCode::OK);
    }

    #[tokio::test]
    async fn verify_email_marks_the_user_verified() {
        let h = spawn().await;
        let token = member_token(&h.app, "verify@example.com").await;

        // Registration captured a verification email; user starts unverified.
        let (_, me_before) = call(&h.app, "GET", "/auth/me", Some(&token), None).await;
        assert_eq!(me_before["verified"], false);

        let verify_link = h
            .email
            .sent()
            .into_iter()
            .find(|e| e.kind == "verify" && e.to == "verify@example.com")
            .expect("verification email captured")
            .link;
        let raw = token_in_link(&verify_link);

        assert_eq!(
            call(
                &h.app,
                "POST",
                "/auth/verify-email",
                None,
                Some(json!({"token": raw}))
            )
            .await
            .0,
            StatusCode::NO_CONTENT
        );

        let (_, me_after) = call(&h.app, "GET", "/auth/me", Some(&token), None).await;
        assert_eq!(me_after["verified"], true);
    }

    #[tokio::test]
    async fn forgot_then_reset_password_is_single_use() {
        let h = spawn().await;
        member_token(&h.app, "reset@example.com").await;

        // forgot-password is always 202 (no enumeration), even for unknown emails.
        assert_eq!(
            call(
                &h.app,
                "POST",
                "/auth/forgot-password",
                None,
                Some(json!({"email": "ghost@example.com"}))
            )
            .await
            .0,
            StatusCode::ACCEPTED
        );
        assert_eq!(
            call(
                &h.app,
                "POST",
                "/auth/forgot-password",
                None,
                Some(json!({"email": "reset@example.com"}))
            )
            .await
            .0,
            StatusCode::ACCEPTED
        );

        let reset_link = h
            .email
            .sent()
            .into_iter()
            .find(|e| e.kind == "reset" && e.to == "reset@example.com")
            .expect("reset email captured")
            .link;
        let raw = token_in_link(&reset_link);

        // Valid reset → 204; the new password logs in.
        assert_eq!(
            call(
                &h.app,
                "POST",
                "/auth/reset-password",
                None,
                Some(json!({"token": raw, "new_password": "brandnew123"})),
            )
            .await
            .0,
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            call(
                &h.app,
                "POST",
                "/auth/login",
                None,
                Some(json!({"email": "reset@example.com", "password": "brandnew123"})),
            )
            .await
            .0,
            StatusCode::OK
        );

        // Reusing the same token fails (single-use).
        assert_eq!(
            call(
                &h.app,
                "POST",
                "/auth/reset-password",
                None,
                Some(json!({"token": raw, "new_password": "another123"})),
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
}
