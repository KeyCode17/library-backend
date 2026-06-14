# ADR — Hexagonal clean architecture per context

| | |
|---|---|
| Status | Accepted |
| Date | 2026-06-14 |
| Owner | _TODO: assign_ |


## Context
Multiple bounded contexts with shared discipline; team already uses an Axum clean-arch
skill.

## Decision
Each context is a library crate with `domain / application / infrastructure / presentation`
layers. Dependency rule: presentation → application → domain; infrastructure implements
domain traits. Domain has no framework imports.

## Consequences
- Use cases are unit-testable with fake repos (no DB/HTTP).
- `gateway` binary is the composition root; `iam/catalog/lending` are libraries.
