# PLAN — Backend Implementation

| | |
|---|---|
| Status | Draft |
| Date | 2026-06-14 |
| Owner | _TODO: assign_ |


## Milestones
1. **M0 — Skeleton.** Workspace (`apps/*`), `iam` context, gateway boot, migrations, seed.
2. **M1 — Catalog + Lending.** Core CRUD contexts + contract endpoints.
3. **M2 — Recommender.** Pure crate + `/recommend` endpoint + `recommender-ffi` + build.sh.
4. **M3 — Chat.** WebSocket delivery + persisted history.
5. **M4 — Notification.** Scheduler + FCM push.
6. **M5 — Hardening.** Authz coverage, observability, CI drift gate.

## Sequencing rules
- Contract change precedes implementation in all three repos.
- `recommender` (pure) before `recommender-ffi`; `build.sh` before android can consume.
- `gateway` composition root assembled last per milestone.

## Cross-repo handoffs
- Publish contract → web/android regen types.
- Run build.sh → publish AAR/Swift Package → android consumes.
