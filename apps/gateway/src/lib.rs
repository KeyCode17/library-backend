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
        // `Secure` cookies only in production (dev is plain http).
        cookie_secure: iam::infrastructure::config::is_production(),
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
    use axum::http::header::{AUTHORIZATION, CONTENT_TYPE, COOKIE, SET_COOKIE};
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use persistence::sea_orm::ConnectionTrait;
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

    /// Like `call`, but also sends an optional `Cookie` header and surfaces the
    /// response `Set-Cookie` header — needed to exercise the cookie auth path.
    async fn raw_call(
        app: &Router,
        method: &str,
        uri: &str,
        bearer: Option<&str>,
        cookie: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Option<String>, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(token) = bearer {
            builder = builder.header(AUTHORIZATION, format!("Bearer {token}"));
        }
        if let Some(cookie) = cookie {
            builder = builder.header(COOKIE, cookie);
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
        let set_cookie = response
            .headers()
            .get(SET_COOKIE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
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
        (status, set_cookie, value)
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
    async fn text_search_matches_title_and_author_case_insensitively() {
        let h = spawn().await;

        // Title match ("Clean Code"), case-insensitive.
        let (status, by_title) = call(&h.app, "GET", "/books?q=CLEAN", None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(by_title["pagination"]["total"].as_u64().expect("total") >= 1);
        assert!(by_title["data"]
            .as_array()
            .expect("data")
            .iter()
            .any(|book| book["title"] == "Clean Code"));

        // Author match ("Robert C. Martin") via a substring.
        let (_, by_author) = call(&h.app, "GET", "/books?q=martin", None, None).await;
        assert!(by_author["pagination"]["total"].as_u64().expect("total") >= 1);
        assert!(by_author["data"]
            .as_array()
            .expect("data")
            .iter()
            .all(|book| book["author"].as_str().expect("author").contains("Martin")));

        // No match → empty page, still 200.
        let (empty_status, empty) =
            call(&h.app, "GET", "/books?q=zzz-no-such-book", None, None).await;
        assert_eq!(empty_status, StatusCode::OK);
        assert_eq!(empty["pagination"]["total"], 0);
        assert_eq!(empty["data"].as_array().expect("data").len(), 0);

        // Combinable with the shelf finder.
        let (_, combined) = call(&h.app, "GET", "/books?q=code&shelf=Tech", None, None).await;
        assert!(combined["data"]
            .as_array()
            .expect("data")
            .iter()
            .all(|book| book["shelf"] == "Tech"));
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
    async fn alter_migration_upgrades_a_pre_iam_v2_database() {
        init_env();
        let container = PgImage::default().start().await.expect("start postgres");
        let port = container.get_host_port_ipv4(5432).await.expect("host port");
        let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
        let db = persistence::db::connect(&url).await.expect("connect");

        // Simulate a 1.0.0-era database: a `users` table WITHOUT the IAM v2
        // columns, plus a migration ledger that marks the original create-users
        // migration as already applied (so the upgrade can't re-create the table).
        db.execute_unprepared(
            "CREATE TABLE seaql_migrations (\
                 version VARCHAR NOT NULL PRIMARY KEY, \
                 applied_at BIGINT NOT NULL)",
        )
        .await
        .expect("ledger");
        db.execute_unprepared(
            "INSERT INTO seaql_migrations (version, applied_at) \
             VALUES ('m20260615_000002_create_users_table', 0)",
        )
        .await
        .expect("mark create-users applied");
        db.execute_unprepared(
            "CREATE TABLE users (\
                 id UUID PRIMARY KEY, \
                 email VARCHAR NOT NULL UNIQUE, \
                 password_hash VARCHAR NOT NULL, \
                 role VARCHAR NOT NULL)",
        )
        .await
        .expect("legacy users table");
        db.execute_unprepared(
            "INSERT INTO users (id, email, password_hash, role) \
             VALUES ('00000000-0000-4000-8000-0000000000aa', 'legacy@x.test', 'hash', 'member')",
        )
        .await
        .expect("legacy row");

        // Upgrade. The ALTER migration must add the missing columns to the
        // existing table (not fail with "column users.verified does not exist").
        migrate(&db).await.expect("upgrade migrates");

        // New columns now exist and the legacy row carries the defaults.
        let alive = db
            .execute_unprepared(
                "SELECT verified, active, created_at FROM users WHERE email = 'legacy@x.test'",
            )
            .await;
        assert!(alive.is_ok(), "iam v2 columns present after upgrade");

        // Idempotent: running the migrator again is a clean no-op.
        migrate(&db).await.expect("re-migrate is a no-op");

        drop(container);
    }

    #[tokio::test]
    async fn login_sets_session_cookie_and_extractor_accepts_cookie_or_bearer() {
        let h = spawn().await;
        call(
            &h.app,
            "POST",
            "/auth/register",
            None,
            Some(json!({"email": "cookie@example.com", "password": "password123"})),
        )
        .await;

        // Login returns the bearer token in the body AND sets the session cookie.
        let (login_status, set_cookie, login_body) = raw_call(
            &h.app,
            "POST",
            "/auth/login",
            None,
            None,
            Some(json!({"email": "cookie@example.com", "password": "password123"})),
        )
        .await;
        assert_eq!(login_status, StatusCode::OK);
        let bearer = login_body["token"].as_str().expect("body token").to_owned();
        let cookie = set_cookie.expect("Set-Cookie present on login");
        assert!(cookie.starts_with("session="));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        // Dev (non-production) must NOT mark the cookie Secure.
        assert!(!cookie.contains("Secure"));
        let session_pair = cookie.split(';').next().expect("cookie pair").to_owned();

        // Protected endpoint via the bearer header.
        let (via_bearer, _, _) =
            raw_call(&h.app, "GET", "/auth/me", Some(&bearer), None, None).await;
        assert_eq!(via_bearer, StatusCode::OK);

        // Same endpoint via the cookie alone.
        let (via_cookie, _, me_body) =
            raw_call(&h.app, "GET", "/auth/me", None, Some(&session_pair), None).await;
        assert_eq!(via_cookie, StatusCode::OK);
        assert_eq!(me_body["email"], "cookie@example.com");

        // Neither credential → 401.
        let (anon, _, _) = raw_call(&h.app, "GET", "/auth/me", None, None, None).await;
        assert_eq!(anon, StatusCode::UNAUTHORIZED);

        // Logout clears the cookie (Max-Age=0).
        let (logout_status, logout_cookie, _) =
            raw_call(&h.app, "POST", "/auth/logout", None, None, None).await;
        assert_eq!(logout_status, StatusCode::NO_CONTENT);
        let cleared = logout_cookie.expect("Set-Cookie present on logout");
        assert!(cleared.starts_with("session="));
        assert!(cleared.contains("Max-Age=0"));
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

    #[tokio::test]
    async fn concurrent_admin_removals_keep_at_least_one_admin() {
        let h = spawn().await;
        let admin_a = admin_token(&h.app).await;

        // Promote a second admin so there are exactly two.
        let (created, b_body) = call(
            &h.app,
            "POST",
            "/users",
            Some(&admin_a),
            Some(
                json!({"email": "admin-b@example.com", "password": "password123", "role": "admin"}),
            ),
        )
        .await;
        assert_eq!(created, StatusCode::CREATED);
        let b_id = b_body["id"].as_str().expect("b id").to_owned();
        let (_, a_me) = call(&h.app, "GET", "/auth/me", Some(&admin_a), None).await;
        let a_id = a_me["id"].as_str().expect("a id").to_owned();

        let (_, b_login) = call(
            &h.app,
            "POST",
            "/auth/login",
            None,
            Some(json!({"email": "admin-b@example.com", "password": "password123"})),
        )
        .await;
        let admin_b = b_login["token"].as_str().expect("b token").to_owned();

        // Each admin tries to delete the other, simultaneously. The transactional
        // guard must let exactly one through (the other would drop admins to zero).
        let (app1, app2) = (h.app.clone(), h.app.clone());
        let task_a = tokio::spawn(async move {
            call(
                &app1,
                "DELETE",
                &format!("/users/{b_id}"),
                Some(&admin_a),
                None,
            )
            .await
            .0
        });
        let task_b = tokio::spawn(async move {
            call(
                &app2,
                "DELETE",
                &format!("/users/{a_id}"),
                Some(&admin_b),
                None,
            )
            .await
            .0
        });
        let s_a = task_a.await.expect("task a");
        let s_b = task_b.await.expect("task b");

        let mut statuses = [s_a, s_b];
        statuses.sort_by_key(|s| s.as_u16());
        // Exactly one deletion succeeds; the other is refused as the last admin.
        assert_eq!(statuses, [StatusCode::NO_CONTENT, StatusCode::CONFLICT]);
    }
}
