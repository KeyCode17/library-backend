# FSD — Backend Behaviour

| | |
|---|---|
| Status | Draft |
| Date | 2026-06-14 |
| Owner | _TODO: assign_ |


## 1. Architecture
Hexagonal clean architecture per context: `domain → application → infrastructure /
presentation`. Axum 0.8 + SeaORM + Postgres. See ADR-0002.

## 2. Contexts & responsibilities
- `iam`: users, roles, permissions, JWT (Argon2). REST.
- `catalog`: book entity incl. `shelf`, `row`; book-finder lookup. REST.
- `lending`: loan lifecycle (borrow → due → return → approve). REST.
- `chat`: history persisted (SeaORM); delivery via WebSocket + broadcast (ADR-0006).
- `notification`: background scheduler checks due dates; push via FCM (ADR-0006).

## 3. Recommendation
`POST /recommend` — input: user prefs + candidate set (or server-fetched candidates);
output: ranked book IDs. Logic in `recommender` (pure, sync), invoked by `gateway`.
The same crate is wrapped by `recommender-ffi` for on-device use.

## 4. Errors
Typed domain errors map to a flat HTTP error contract (status + JSON `{ "error" }`).
Internal errors logged via tracing, generic 500 returned.

## 5. Contract
Every endpoint is described in `contract/openapi.yaml`. Changing an endpoint = change the
contract first, then implement; CI fails on drift.
