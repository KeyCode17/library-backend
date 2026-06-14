# library-backend — docs

Documentation set for the backend (Rust / Axum + Postgres) **and** the shared core
(`recommender` + `recommender-ffi`). The OpenAPI contract that the other two repos
generate types from also lives in this repo (`contract/openapi.yaml`).

```
docs/
├── sow/   Statement of Work — scope, deliverables, boundaries
├── prd/   Product Requirements — what & why
├── fsd/   Functional Spec — how it behaves, endpoint by endpoint
├── plan/  Implementation Plan — milestones, sequencing
└── adr/   Architecture Decision Records — one decision per file
```
File convention: `xxx-{slug}-{repo}.md` (e.g. `001-initial-scope-backend.md`); ADRs `NNNN-{slug}-backend.md`.
