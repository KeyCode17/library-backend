# library-backend

Rust/Axum backend for the library project. **v1.2.0 — completeness: idempotent upgrade
migration, an active-loan unique-index backstop, `GET /books?q=` search, and httpOnly+SameSite
session-cookie auth (alongside bearer).** Deployment requires `DATABASE_URL`, `IAM_JWT_SECRET`,
and `RESEND_API_KEY` (all fail-closed in production), plus FCM config for push.

## What's built

A booting Cargo workspace with the pre-push quality gate wired and the first feature slice
(T-001 catalog listing). The API is contract-first: every endpoint is defined in
[`contract/openapi.yaml`](contract/openapi.yaml), the single source of truth all three repos
derive from (ADR 0004).

- **Workspace** — `members = ["apps/*"]`, single version line in `[workspace.package]`.
- **`apps/gateway`** — the composition root (per
  [ADR 0002](docs/adr/0002-hexagonal-clean-architecture-backend.md)). Boots Axum, injects
  concrete adapters into each context, and merges their routers.
- **`apps/catalog`** — the catalog bounded context (hexagonal:
  `domain / application / infrastructure / presentation`). Lists books with a shelf/row/ISBN
  finder; persisted in Postgres (see Persistence below).
- **`apps/persistence`** — shared SeaORM entities (the schema source of truth) + the Postgres
  connection pool. `migration` derives the DDL from these entities.
- **`apps/iam`** — auth, roles, permissions (hexagonal). JWT bearer tokens, Argon2id password
  hashing, RBAC (`admin` / `librarian` / `member`). Authorization is enforced server-side in
  the use cases, not just at the edge.
- **`apps/lending`** — the loan lifecycle (`borrowed → returned → approved`, hexagonal). Reuses
  IAM's bearer extractor + RBAC; members act on their own loans, staff approve. Book
  availability is reached through a `BookGateway` port — the gateway bridges it to `catalog`, so
  the contexts stay decoupled while a borrow flips the book unavailable.
- **`apps/recommender`** — a **pure, sync** decision-tree ranking crate (ADR 0005): no I/O,
  HTTP, FFI, or async. The gateway calls it directly for `POST /recommend`.
- **`apps/recommender-ffi`** — a thin UniFFI 0.28 (proc-macro) `cdylib` wrapping `recommender`
  for Kotlin/Swift. No logic of its own — the server and the phone rank identically.
- **`apps/chat`** — group chat over WebSocket (ADR 0006). History in an in-memory store;
  live delivery via a per-room broadcast hub. The WS upgrade and REST history both authenticate
  with the IAM JWT.
- **`apps/notification`** — due-date reminders pushed via FCM (ADR 0006). A background scheduler
  (tokio interval) scans active-loan due dates and produces due-soon/overdue reminders, pushing
  through a `PushSender` port. The real FCM (HTTP v1) adapter is credential-gated; a fake records
  pushes in tests. The scheduler reads loans via a `LoanSource` port the gateway bridges to
  `lending`, keeping the contexts decoupled.
- **`apps/migration`** — SeaORM schema/migrations. The `books`, `users`, `loans`, `chat_messages`,
  `devices`, and `reminders` table DDL is generated from the entities
  (`Schema::create_table_from_entity`), per the generate-migrations rule.

  | Method | Path                            | Auth   | Response                                          |
  |--------|---------------------------------|--------|---------------------------------------------------|
  | `GET`  | `/healthz`                      | public | `200 {"status":"ok"}`                             |
  | `GET`  | `/books?q=&shelf=&row=&isbn=`   | public | `200 { data: Book[], pagination }` (text + finder)|
  | `GET`  | `/books/{id}`                   | public | `200 Book` / `404`                                |
  | `POST` | `/auth/register`                | public | `201 Principal` (creates a `member`)              |
  | `POST` | `/auth/login`                   | public | `200 AuthToken` (JWT, also sets `session` cookie) |
  | `POST` | `/auth/logout`                  | public | `204` (clears the `session` cookie)               |
  | `GET`  | `/auth/me`                      | bearer/cookie | `200 Principal` / `401`                    |
  | `POST` | `/users/{id}/roles`             | admin  | `200 Principal` / `401` / `403` / `404`           |
  | `POST` | `/loans`                        | bearer | `201 Loan` / `401` / `404` / `409` (borrow)       |
  | `GET`  | `/loans`                        | bearer | `200 LoanList` (member: own; staff: all)          |
  | `POST` | `/loans/{id}/return`            | bearer | `200 Loan` / `401` / `403` / `404` (owner/staff)  |
  | `POST` | `/loans/{id}/approve`           | staff  | `200 Loan` / `401` / `403` / `404` (staff only)   |
  | `POST` | `/recommend`                    | public | `200 { ranked: [uuid] }` (prefs in body)          |
  | `GET`  | `/chat/rooms/{room}/messages`   | bearer | `200 ChatMessageList` / `401`                      |
  | `GET`  | `/ws/chat?room=&token=`         | token  | WebSocket upgrade (`101`) / `401`                 |
  | `POST` | `/notifications/devices`        | bearer | `201 Device` / `400` / `401` (register FCM token) |
  | `GET`  | `/notifications`                | bearer | `200 NotificationList` / `401` (reminder history) |

  `401` = unauthenticated; `403` = authenticated but lacking the role/ownership. Catalog stays
  public. Borrowing flips a book unavailable; returning flips it back. The due-date scheduler is
  internal (no endpoint) — it runs on a tokio interval and pushes reminders via FCM.

  **Catalog search:** `GET /books?q=<text>` does a case-insensitive substring match over
  title/author/ISBN, combinable with the `shelf`/`row`/`isbn` finder and pagination. An empty
  result is a `200` with an empty page, not a `404`.

  **Auth transports:** `POST /auth/login` returns the JWT in the body *and* sets it as a
  `session` cookie (`HttpOnly; SameSite=Lax; Path=/; Max-Age=<ttl>`; `Secure` is added only in
  production). Protected routes accept **either** `Authorization: Bearer <jwt>` (android — takes
  precedence) **or** the `session` cookie (web). `POST /auth/logout` expires the cookie
  (`Max-Age=0`); bearer clients just discard their token. **CSRF posture:** `SameSite=Lax` plus
  JSON-only, non-GET state changes mitigates CSRF without a double-submit token (kept simple).
  **CORS:** none is configured — the web app is first-party and proxies `/api` to the gateway, so
  requests are same-origin. If a future cross-origin client with credentials is needed, set
  `Access-Control-Allow-Credentials` against a specific origin (never `*`).

  **Chat WS:** connect to `GET /ws/chat?room=<room>&token=<jwt>`. Auth is resolved in order: the
  `token` query param (browsers can't set headers on a WS handshake), then `Authorization: Bearer`,
  then the `session` cookie (web — its JWT is httpOnly, so there's no JS token for the query param).
  Send `ChatSend` (`{ body }`) frames; receive `ChatMessage` frames broadcast to every connection in
  the room. Each message is persisted to history.

## Mobile artifact (`build.sh`)

`./build.sh` cross-compiles `recommender-ffi` to Android (via `cargo ndk`, scoped to that crate
only) and generates the UniFFI Kotlin bindings. Outputs:

- `build/recommender/recommender.aar` — native libs (`jni/<abi>/librecommender_ffi.so`)
- `build/recommender/kotlin/` — generated Kotlin bindings (the android module adds these as
  source; it already provides the JNA dependency UniFFI needs)

It is idempotent and **fails loudly** if `cargo ndk` / the Android NDK is missing. The
orchestrator runs it as the cross-repo step; the android repo consumes the AAR.

## Configuration

Secrets are config-driven — nothing is hardcoded or committed.

| Env var              | Purpose                          | Dev fallback if unset                                |
|----------------------|----------------------------------|------------------------------------------------------|
| `DATABASE_URL`       | Postgres DSN (all contexts)      | `postgres://postgres:postgres@localhost:5432/postgres` |
| `IAM_JWT_SECRET`     | JWT signing secret (HS256)       | ephemeral random secret (warns; not stable)          |
| `IAM_TOKEN_TTL_SECS` | token lifetime                   | `3600`                                                |
| `IAM_ADMIN_EMAIL`    | seeded admin email               | `admin@library.local`                                 |
| `IAM_ADMIN_PASSWORD` | seeded admin password (dev seed) | random password generated + printed at boot           |
| `FCM_PROJECT_ID`     | Firebase project for FCM v1 push | push is a logged no-op                                |
| `FCM_ACCESS_TOKEN`   | FCM v1 OAuth2 bearer             | push is a logged no-op                                |
| `RESEND_API_KEY`     | Resend API key (transactional email) | dev: email is a logged no-op; **prod: fatal**    |
| `RESEND_FROM`        | Sender address                   | `onboarding@resend.dev`                               |
| `APP_PUBLIC_URL`     | base for verification/reset links | `http://localhost:8080`                              |

**Production fails closed:** with `APP_ENV`/`RUST_ENV` = `production`, a missing `DATABASE_URL`,
`IAM_JWT_SECRET`, or `RESEND_API_KEY` is fatal (the gateway refuses to start) rather than falling
back to a dev default.

**IAM v2:** admin user management (`GET`/`POST /users`, `PATCH`/`DELETE /users/{id}`,
`POST /users/{id}/roles`) with last-admin lockout safety; self-service (`POST /auth/change-password`,
`PATCH`/`DELETE /auth/me`); email verification (`POST /auth/verify-email`) on register and password
reset (`POST /auth/forgot-password` → always `202`, `POST /auth/reset-password`). Email goes through
an `EmailSender` port (Resend adapter, credential-gated + a fake for tests); reset/verify tokens are
random, stored hashed (SHA-256), single-use, and time-limited (reset 1h, verify 24h).

## Persistence

Every context persists to **Postgres via SeaORM** (ADR 0003). The schema lives as SeaORM entities
in `apps/persistence` (the single source of truth); `apps/migration` derives the DDL from them and
runs at startup. Each context's `infrastructure` has a `SeaOrm*Repository` adapter behind its
unchanged domain port; the in-memory adapters remain only for the contexts' DB-free unit tests.
DB-backed integration tests use an ephemeral Postgres via **testcontainers** (Docker required).

Borrow is **atomic**: a single conditional `UPDATE books SET available=false WHERE id=? AND
available=true` claims the book; concurrent borrows of the same book yield exactly one active loan
(the loser gets `409`).

Set `IAM_JWT_SECRET` and `IAM_ADMIN_PASSWORD` in any real deployment. With them unset the
gateway still boots for local dev, logging a warning (and the generated admin password).

When `FCM_*` is unset the notification scheduler still runs but pushes no-op (logged). Real FCM
delivery needs a Firebase service account to mint `FCM_ACCESS_TOKEN` — a deployment concern
tracked for 0.9, like the JWT secret.

## Run

```bash
# needs a Postgres (DATABASE_URL); migrations + seed run at startup
cargo run -p gateway                 # listens on 0.0.0.0:8080 (override with PORT)
curl localhost:8080/healthz          # -> {"status":"ok"}
curl 'localhost:8080/books'          # -> { "data": [ ...8 seeded books... ], "pagination": {...} }
curl 'localhost:8080/books?page=2&page_size=3'   # paginated
curl 'localhost:8080/books?isbn=978-0132350884'  # resolve a scanned ISBN -> one book

# auth
curl -X POST localhost:8080/auth/register -H 'content-type: application/json' \
  -d '{"email":"a@b.com","password":"password123"}'                 # -> 201 member
TOKEN=$(curl -s -X POST localhost:8080/auth/login -H 'content-type: application/json' \
  -d '{"email":"a@b.com","password":"password123"}' | jq -r .token)
curl localhost:8080/auth/me -H "authorization: Bearer $TOKEN"        # -> current principal
```

## Test

```bash
cargo test --workspace
```

## Quality gate (lefthook)

Git hooks are managed by [lefthook](https://lefthook.dev/) (never husky). Install once:

```bash
lefthook install
```

- **pre-commit** — `cargo fmt -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`
- **pre-push** — the above plus `cargo test --workspace`

Clippy runs with `-D warnings` (warnings fail the build). Never bypass with `--no-verify`.
