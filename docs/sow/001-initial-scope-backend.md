# SOW — Backend & Core Initial Scope

| | |
|---|---|
| Status | Draft |
| Date | 2026-06-14 |
| Owner | _TODO: assign_ |


## 1. Objective
Deliver the HTTP backend for the library platform and the shared Rust core. The backend
owns all persistent, multi-user data; the core owns the recommendation logic shared with
mobile.

## 2. In scope
- Bounded contexts: `iam` (auth/roles), `catalog` (books + shelf location), `lending`
  (borrow/return/due dates), `chat` (group chat), `notification` (reminders).
- `recommender` (pure decision-tree crate) + `recommender-ffi` (UniFFI cdylib for mobile).
- `gateway` Axum binary (composition root) exposing `/recommend` and the context routers.
- `bootstrap` seed binary; `migrations` (sea-orm-migration).
- `contract/openapi.yaml` — the single source of truth consumed by web + android.
- `build.sh` cross-compiling `recommender-ffi` for Android/iOS.

## 3. Out of scope
- Frontend UI (`library-web`, `library-android`).
- On-device storage (Room) — owned by the Android app.
- Edge/Cloudflare deployment — explicitly rejected (see ADR-0003).

## 4. Deliverables
1. Running Axum service with the five contexts.
2. Published OpenAPI contract.
3. Published mobile bindings artifact (AAR / Swift Package) from `build.sh`.
4. Migrations + idempotent seed.

## 5. Acceptance
- `cargo test` green (pure use cases unit-tested with fake repos).
- Contract validates; web + android type-gen succeeds against it.
- `recommender` compiles for native targets used by gateway and ffi.
