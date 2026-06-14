# ADR — Chat and notification are not the REST CRUD pattern

| | |
|---|---|
| Status | Accepted |
| Date | 2026-06-14 |
| Owner | _TODO: assign_ |


## Context
Chat (realtime) and notification (scheduled) don't fit the request-response skill template.

## Decision
Both live as contexts in the same backend but use their own delivery:
- `chat`: persistence follows the skill (SeaORM history), delivery is WebSocket + a
  broadcast/connection registry. Auth via `iam`.
- `notification`: persistence follows the skill, delivery is a background scheduler
  (`tokio` interval) + FCM push.

## Consequences
- Don't force WebSocket/scheduler logic into the REST handler template.
- Can be extracted into separate services later if load diverges.
