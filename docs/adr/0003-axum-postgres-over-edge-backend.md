# ADR — Axum + Postgres, not Cloudflare edge

| | |
|---|---|
| Status | Accepted |
| Date | 2026-06-14 |
| Owner | _TODO: assign_ |


## Context
Considered deploying the backend to Cloudflare Workers (edge/WASM).

## Decision
Use a conventional Axum + Postgres server (native), not edge.

## Consequences
- Chat (WebSocket) and notification (background scheduler) are first-class on Axum;
  they are awkward on stateless Workers (would require Durable Objects).
- `recommender` stays native-pure; no wasm32 constraint, no `recommender-wasm` crate.
- Web gets recommendations via REST `/recommend`, not browser WASM.
