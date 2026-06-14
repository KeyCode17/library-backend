# ADR — Split pure `recommender` from `recommender-ffi`

| | |
|---|---|
| Status | Accepted |
| Date | 2026-06-14 |
| Owner | _TODO: assign_ |


## Context
Putting `#[uniffi::export]` directly on the recommendation crate would drag UniFFI
scaffolding into the backend and block any future wasm32 target.

## Decision
Two crates: `recommender` (pure: domain + application, deps = shared-kernel only) and
`recommender-ffi` (cdylib wrapper, UniFFI). Backend imports the pure crate; mobile imports
the ffi crate.

## Consequences
- `recommender` is `cargo test`-able without any mobile toolchain.
- `build.sh` cross-compiles only `recommender-ffi` (`-p recommender-ffi`), never the whole
  workspace (sqlx/tokio won't cross-compile to Android).
