# ADR — Merge the Rust core into the backend repo

| | |
|---|---|
| Status | Accepted |
| Date | 2026-06-14 |
| Owner | _TODO: assign_ |


## Context
`recommender` is consumed by both the backend (`/recommend`) and mobile (via UniFFI). It
could live in its own repo or inside the backend.

## Decision
Keep `recommender` + `recommender-ffi` inside `library-backend` as workspace crates.

## Consequences
- Backend imports `recommender` via a `path` dependency — no cross-repo git dep, one
  `Cargo.lock`, uniform versions.
- Mobile consumes the FFI **artifact** (AAR/Swift Package) produced by `build.sh`, so it
  never clones the backend repo.
- Revisit only if `recommender` becomes a standalone public library.
