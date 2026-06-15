# ADR — Hand-written SQL for the active-loan partial unique index

| | |
|---|---|
| Status | Accepted |
| Date | 2026-06-15 |
| Owner | backend |

## Context

The `generate-migrations` rule requires every migration to be derived from the SeaORM
entity (no hand-written SQL): the entity is the schema source of truth, and the migration
tool emits the DDL.

Borrow atomicity (ADR 0003 / lending) is enforced at the application layer by an atomic
conditional claim (`UPDATE books SET available=false WHERE id=? AND available=true`). We want
a **database backstop** that makes "≤ 1 active loan per book" a hard invariant even if a
future code path bypassed the claim:

```sql
CREATE UNIQUE INDEX uniq_active_loan_per_book ON loans (book_id) WHERE status = 'borrowed';
```

This is a **partial** (predicated) unique index. SeaORM's schema builder
(`Schema::create_table_from_entity`, `Index::create()`) cannot express a `WHERE` predicate on
an index, and the entity model has no representation for it. There is no entity-derived way to
produce this DDL.

## Decision

Create this **one** index via a hand-written-SQL migration
(`m20260615_000008_active_loan_unique_index`, `execute_unprepared`). This is an **authorized,
deliberate exception** to the generate-migrations rule — not a weakening of it.

Scope of the exception is strictly limited to:

- this single partial unique index, and
- any future index/constraint that is genuinely inexpressible via the SeaORM schema builder.

Everything else stays entity-derived. Table/column DDL (including the IAM v2 `ALTER TABLE …
ADD COLUMN IF NOT EXISTS`) continues to use the typed SeaORM builders, not raw SQL.

## Consequences

- The "one active borrow per book" invariant is guaranteed by the database, not just the
  application — defense in depth on top of the conditional claim.
- The hand-written SQL is idempotent (`CREATE UNIQUE INDEX IF NOT EXISTS` / `DROP INDEX IF
  EXISTS`) and applies cleanly to both fresh and upgraded databases.
- Reviewers should treat any *new* hand-written-SQL migration as requiring its own
  justification; this ADR does not blanket-authorise raw SQL.
