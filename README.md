# library-backend

Rust/Axum backend for the library project. **Version 0.1.0.**

## What's built (M0 skeleton)

A booting Cargo workspace with the pre-push quality gate wired. No feature code yet —
the catalog/contract endpoints land in later milestones (see
[`docs/plan/001-implementation-plan-backend.md`](docs/plan/001-implementation-plan-backend.md)).

- **Workspace** — `members = ["apps/*"]`, single version line in `[workspace.package]`.
- **`apps/gateway`** — the composition root (per
  [ADR 0002](docs/adr/0002-hexagonal-clean-architecture-backend.md)). An Axum server that
  exposes one route:

  | Method | Path       | Response                      |
  |--------|------------|-------------------------------|
  | `GET`  | `/healthz` | `200 {"status":"ok"}`         |

Feature contexts (`iam`, `catalog`, `lending`, `recommender`) are added as library crates
under `apps/*` and merged into the gateway router as they come online.

## Run

```bash
cargo run -p gateway          # listens on 0.0.0.0:8080 (override with PORT)
curl localhost:8080/healthz   # -> {"status":"ok"}
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
