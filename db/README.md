# Database artifacts

```
db/
├── migrations/<engine>/     structure only — versioned, run once, never edited after release
└── seeds/<engine>/
    ├── reference/           data the application needs to function
    └── development/         dev convenience — never run in production
```

Laravel's split, with one extra distinction: `database/migrations` → `db/migrations`,
`database/seeders` → `db/seeds`, and seeds are further divided by whether production
needs them.

## Migrations vs seeds

| | Migrations | Seeds |
|---|---|---|
| Contains | DDL only | INSERTs only |
| Versioned | yes, `_sqlx_migrations` tracks them | no |
| Run | once | every deploy, re-runnable |
| Editing after release | never — the checksum is enforced | fine, they are idempotent |

## reference vs development

`reference/` is **not optional sample data**. `RoleName::SUPERADMIN` and every
permission name checked in a handler are meaningless without those rows — a database
with the schema but without them has a working API and a silently broken authorization
system. Deployment runs it on every release.

`development/` grants superadmin to whoever holds the lowest user id. That is a
convenience for a dev box and an incident in production, which is why it is a separate
set that requires an explicit flag.

## Commands

```bash
./scripts/migrate.sh              # apply migrations for the engine in DATABASE_URL
./scripts/migrate.sh info         # what is applied and what is pending
./scripts/migrate.sh revert       # roll back the most recent migration

cargo run -p infrastructure --bin seed           # reference only
cargo run -p infrastructure --bin seed -- --dev  # reference + development
```

Both pick the engine folder from the `DATABASE_URL` scheme, so there is no second
setting that can disagree with the connection string.

## What the per-engine layout does and does not buy you

It makes the *files* engine-agnostic. It does not make the *application*
engine-agnostic, and the gap is larger than the folder suggests:

- **Oracle has no `sqlx` driver.** sqlx supports Postgres, MySQL and SQLite. Oracle
  means the `oracle` crate, which links Oracle Instant Client — a native C library that
  breaks the static musl build and the "no shipped native libraries" constraint in
  `SETUP.md`. That is a deliberate architectural reversal, not a driver swap.
- **The adapters are Postgres-typed**, not merely Postgres-dialected: `PgPool`,
  `PgPoolOptions`, and every `query_as` are bound to the Postgres backend.
- **Queries differ past the DDL.** Placeholders (`$1` / `?` / `:1`), the `= ANY($1)`
  array bind that avoids the N+1 in `PgUserRepository` and has no MySQL equivalent, and
  `ON CONFLICT` / `INSERT IGNORE` / `MERGE`.
- **The architecture bets on Postgres.** `SETUP.md` commits to `COPY` writes and jsonb
  extraction in the projection. `COPY` is Postgres-only.

The good news is where the work would land. Because `UserRepository` and the other
ports are declared in `domain` with no SQL in them, a second engine is a second adapter
set under `infrastructure/persistence/` plus a choice at startup — the same place the
seed runner already makes it. `domain`, `application` and `api` would not change at
all. The layering already did the expensive part; nothing here is blocked by structure.
