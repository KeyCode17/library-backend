# PRD — Backend & Core

| | |
|---|---|
| Status | Draft |
| Date | 2026-06-14 |
| Owner | _TODO: assign_ |


## 1. Problem
A library needs a multi-user system of record (catalog, lending, members) plus shared
recommendation logic usable identically on server and mobile.

## 2. Users
Librarians (staff), members (borrowers), platform admins.

## 3. Capabilities
- Authenticate users; assign roles/permissions (`iam`).
- Manage books and physical shelf location — rack/row (`catalog`).
- Record borrow/return, due dates, staff approval (`lending`).
- Group chat by event / book category / ask-a-librarian (`chat`).
- Borrowing reminders + push (`notification`).
- Book recommendations via a decision tree, callable at `/recommend` (`recommender`).

## 4. Non-functional
- Native-deployable (server), no edge runtime constraints.
- Recommendation logic must be reusable on-device without rewrite.
- All inputs validated at the HTTP boundary; auth enforced server-side (authoritative).

## 5. Success metrics
- p95 read latency < 200ms; recommendation call < 50ms server-side.
- Zero contract drift between repos (CI-enforced).
