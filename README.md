# library-backend

Rust/Axum backend for the library project. **v0.4.0 — Lending: borrow/return/due/staff-approve.**

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
  `domain / application / infrastructure / presentation`). Serves the seeded book list via
  an in-memory repository (the Postgres adapter lands with the DB wiring).
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
- **`apps/migration`** — SeaORM schema/migrations. The `books`, `users`, and `loans` table DDL
  is generated from the entities (`Schema::create_table_from_entity`), per the
  generate-migrations rule.

  | Method | Path                  | Auth   | Response                                          |
  |--------|-----------------------|--------|---------------------------------------------------|
  | `GET`  | `/healthz`            | public | `200 {"status":"ok"}`                             |
  | `GET`  | `/books`              | public | `200 { data: Book[], pagination }`                |
  | `GET`  | `/books/{id}`         | public | `200 Book` / `404`                                |
  | `POST` | `/auth/register`      | public | `201 Principal` (creates a `member`)              |
  | `POST` | `/auth/login`         | public | `200 AuthToken` (JWT) / `401`                     |
  | `GET`  | `/auth/me`            | bearer | `200 Principal` / `401`                           |
  | `POST` | `/users/{id}/roles`   | admin  | `200 Principal` / `401` / `403` / `404`           |
  | `POST` | `/loans`              | bearer | `201 Loan` / `401` / `404` / `409` (borrow)       |
  | `GET`  | `/loans`              | bearer | `200 LoanList` (member: own; staff: all)          |
  | `POST` | `/loans/{id}/return`  | bearer | `200 Loan` / `401` / `403` / `404` (owner/staff)  |
  | `POST` | `/loans/{id}/approve` | staff  | `200 Loan` / `401` / `403` / `404` (staff only)   |
  | `POST` | `/recommend`          | public | `200 { ranked: [uuid] }` (prefs in body)          |

  `401` = unauthenticated; `403` = authenticated but lacking the role/ownership. Catalog stays
  public. Borrowing flips a book unavailable; returning flips it back.

## Mobile artifact (`build.sh`)

`./build.sh` cross-compiles `recommender-ffi` to Android (via `cargo ndk`, scoped to that crate
only) and generates the UniFFI Kotlin bindings. Outputs:

- `build/recommender/recommender.aar` — native libs (`jni/<abi>/librecommender_ffi.so`)
- `build/recommender/kotlin/` — generated Kotlin bindings (the android module adds these as
  source; it already provides the JNA dependency UniFFI needs)

It is idempotent and **fails loudly** if `cargo ndk` / the Android NDK is missing. The
orchestrator runs it as the cross-repo step; the android repo consumes the AAR.

Remaining contexts (`lending`, `recommender`) are added as crates under `apps/*` and merged
into the gateway router as they come online.

## Configuration (IAM)

Secrets are config-driven — nothing is hardcoded or committed.

| Env var              | Purpose                          | Dev fallback if unset                        |
|----------------------|----------------------------------|----------------------------------------------|
| `IAM_JWT_SECRET`     | JWT signing secret (HS256)       | ephemeral random secret (warns; not stable)  |
| `IAM_TOKEN_TTL_SECS` | token lifetime                   | `3600`                                        |
| `IAM_ADMIN_EMAIL`    | seeded admin email               | `admin@library.local`                         |
| `IAM_ADMIN_PASSWORD` | seeded admin password (dev seed) | random password generated + printed at boot   |

Set `IAM_JWT_SECRET` and `IAM_ADMIN_PASSWORD` in any real deployment. With them unset the
gateway still boots for local dev, logging a warning (and the generated admin password).

## Run

```bash
cargo run -p gateway                 # listens on 0.0.0.0:8080 (override with PORT)
curl localhost:8080/healthz          # -> {"status":"ok"}
curl 'localhost:8080/books'          # -> { "data": [ ...8 seeded books... ], "pagination": {...} }
curl 'localhost:8080/books?page=2&page_size=3'   # paginated

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
