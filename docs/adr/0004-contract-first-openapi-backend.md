# ADR — Contract-first via OpenAPI

| | |
|---|---|
| Status | Accepted |
| Date | 2026-06-14 |
| Owner | _TODO: assign_ |


## Context
Three repos (backend/android/web) and an orchestrator dispatching parallel work risk DTO
drift.

## Decision
`contract/openapi.yaml` in this repo is the single source of truth. Backend implements to
it; web runs `openapi-typescript`; android generates data classes from it.

## Consequences
- CI fails on contract drift.
- A feature touching the API changes the contract first, then the three sides.
