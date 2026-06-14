# library-backend

Rust/Axum backend for the library project. **v0.1.0 — catalog list (GET /books) shipped.**

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
- **`apps/migration`** — SeaORM schema/migrations. The `books` table DDL is generated from
  the `book` entity (`Schema::create_table_from_entity`), per the generate-migrations rule.

  | Method | Path       | Response                                             |
  |--------|------------|------------------------------------------------------|
  | `GET`  | `/healthz` | `200 {"status":"ok"}`                                |
  | `GET`  | `/books`   | `200 { data: Book[], pagination }` — public, no auth |

Remaining contexts (`iam`, `lending`, `recommender`) are added as crates under `apps/*` and
merged into the gateway router as they come online.

## Run

```bash
cargo run -p gateway                 # listens on 0.0.0.0:8080 (override with PORT)
curl localhost:8080/healthz          # -> {"status":"ok"}
curl 'localhost:8080/books'          # -> { "data": [ ...8 seeded books... ], "pagination": {...} }
curl 'localhost:8080/books?page=2&page_size=3'   # paginated
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
