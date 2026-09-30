# Architecture — Layered DDD

The workspace is one crate per **layer**, with bounded contexts as **modules** inside
each layer.

```
crates/
  domain/          entities, value objects, repository ports        no I/O
  application/     use cases                                        depends: domain
  infrastructure/  sqlx adapters implementing the ports             depends: domain
  api/             axum, DTOs, composition root                     depends: all
  stats/           pure numerics (supporting subdomain)             depends: nothing
```

## The dependency rule

Dependencies point **inward only**. `domain` is the centre and depends on no other
workspace crate.

This is not a convention — it is the Cargo graph. `crates/domain/Cargo.toml` does not
list `sqlx`, `axum`, or `tokio`, so a `use sqlx::...` in a domain file is a compile
error, not a code-review comment. Verify it any time:

```bash
cargo tree -i sqlx -e normal --workspace     # must reach infrastructure -> api only
cargo tree -i axum -e normal --workspace     # must reach api only
cargo tree -p domain -e normal --depth 1     # chrono, thiserror, async-trait
```

If a layer seems to need a dependency it isn't allowed, the missing piece is a **port**
in `domain`, not a new entry in that `Cargo.toml`.

## What goes where

| Question | Layer |
|---|---|
| "Can this value legally exist?" | `domain` — value object constructor |
| "What is this thing and what can it do?" | `domain` — entity |
| "What does persistence have to provide?" | `domain` — repository trait (the port) |
| "Which steps make up this operation?" | `application` — a use case |
| "How do we actually read/write it?" | `infrastructure` — an adapter |
| "What status code / JSON shape?" | `api` — `http/error.rs`, `http/dto/` |
| "Which concrete adapter do we use?" | `api` — `state.rs` and `main.rs`, nowhere else |

## One thing per file

**No source file exceeds 150 lines.** Enforced by `scripts/check.sh`, which fails the
build and names any file that crosses it.

The limit is a tripwire, not the goal. A fat file is a symptom: it has quietly absorbed
a second responsibility, and the fix is to name that responsibility and give it its own
file — never to shuffle lines until the count drops. Splitting a coherent 160-line file
into two incoherent 80-line files makes the code worse and satisfies the counter.

Each layer has an obvious seam to split along:

| Layer | One file per |
|---|---|
| `domain` | entity, value object, or port — `user.rs`, `email.rs`, `repository.rs` |
| `application` | use case — `list_users.rs`, `get_user.rs`, one `execute` each |
| `infrastructure` | adapter, with its row struct beside it — `pg_user_repository.rs`, `user_row.rs` |
| `api` | context for handlers and DTOs — `handlers/identity.rs`, `dto/identity.rs` |

So a new use case is a new file, not another `impl` block in an existing one. When
`handlers/identity.rs` outgrows the limit it splits by operation
(`handlers/identity/list_users.rs`); when `domain/identity/` does, the context itself
is probably two contexts.

Inline `#[cfg(test)] mod tests` counts toward the limit — 150 lines leaves room for a
normal test module. When tests genuinely need more, move them to a sibling
`#[path]` module or to `tests/`, and keep the unit under test readable.

The rule covers `.rs` sources. Prose in `docs/` is exempt: a runbook read top to bottom
is not improved by being cut into pieces.

## Two mappings, on purpose

Data crosses two boundaries and changes shape at both:

```
users table  ->  UserRow  ->  User (entity)  ->  UserResponse (DTO)  ->  JSON
                 └ infrastructure ┘              └────── api ──────┘
```

- **`UserRow` -> `User`** lets the table change without the domain noticing, and is the
  only place a stored record is re-validated (`RepositoryError::Corrupt`).
- **`User` -> `UserResponse`** means renaming a domain field is not a breaking API
  change, and entity fields can stay private.

Entity fields *are* private, with getters. Outside its module a `User` can only come
from a constructor that checked its invariants, so no layer above can build one the
database would reject.

## Errors

Each layer has its own error type and translates at the boundary:

```
sqlx::Error  ->  RepositoryError  ->  ApplicationError  ->  HTTP status
                 (domain/shared)     (application)         (api/http/error.rs)
```

`RepositoryError::Backend` boxes the driver error, which is how the domain names a
storage failure without depending on the driver. The match in `api/http/error.rs` is
the single place a failure becomes a status code — add an `ApplicationError` variant
and that match stops compiling until you decide the code.

Startup has its own type, `ConfigError`, which names the offending variable but never
its value — `DATABASE_URL` holds a password and startup errors reach journals and CI
logs.

**`unwrap()` and `expect()` are denied workspace-wide** (`[workspace.lints.clippy]`,
with `clippy.toml` allowing them in tests). There is no `catch` for a panic in a
handler: the task dies, the client gets a bare 500 with no error body, and nothing is
audited. Every failure has to be a `Result` that some layer translates. When a value
genuinely cannot be absent, prove it with `if let` / `let else` rather than asserting
it — and if it truly is unreachable, that is a domain invariant worth encoding in a
type.

## OpenAPI

Documentation is a presentation concern, so it lives entirely in `api`. No `utoipa`
annotation appears in `domain`, `application`, or `infrastructure` — the DTO boundary
is what makes that possible. Confirm it any time:

```bash
grep -rn "utoipa\|ToSchema" crates/domain crates/application crates/infrastructure
# expect: no matches
```

Three pieces:

| File | Holds |
|---|---|
| `http/openapi.rs` | title, description, tags, security schemes — metadata only |
| `#[utoipa::path]` on each handler | that endpoint's params and responses |
| `http/routes.rs` | the single registration of route **and** doc |

`routes.rs` uses `OpenApiRouter`, and `routes!(handler)` reads the method and path
from the handler's own `#[utoipa::path]`. A route cannot be mounted without appearing
in the spec, and cannot appear in the spec without being mounted. `ApiDoc` deliberately
lists no `paths(...)` — they are collected from the router.

The one pairing nothing checks for you: the status codes in a handler's `responses(...)`
and the match arms in `http/error.rs`. Add a status in one place, add it in the other.

```
GET /swagger-ui              interactive UI
GET /api-docs/openapi.json   the spec, generated at startup
./scripts/openapi.sh                                  # same spec, no server, no DB
```

`docs/openapi.json` is committed so response-shape changes show up in review. CI can
enforce it:

```bash
./scripts/openapi.sh && git diff --exit-code docs/openapi.json
```

Both doc routes are gated on `ENABLE_DOCS` (default true) — see `RUNNING.md`. The
`api` crate is a lib plus two bins (`api`, `openapi`) so the spec dump can reuse the
same modules; `default-run = "api"` keeps bare `cargo run -p api` working.

## Adding a bounded context

`mev` is the next one. It is a module in each layer, not a new crate:

1. `crates/domain/src/mev.rs` + `mev/` — entities, value objects, ports
2. `crates/application/src/mev/` — one file per use case
3. `crates/infrastructure/src/persistence/mev/` — row structs + adapters
4. `crates/api/src/http/{dto,handlers}/mev.rs` — DTOs derive `ToSchema`, handlers carry
   `#[utoipa::path(... tag = "mev" ...)]` — plus a `mev_routes()` merged in
   `http/routes.rs` and a `mev` tag in `http/openapi.rs`
5. Wire the adapter in `AppState::build`
6. Regenerate `docs/openapi.json`

Follow `identity` — it is the reference implementation of every step.

Promote a context to its own crate only when its compile time actually hurts; the
module layout moves across unchanged.

## Database artifacts

```
db/migrations/<engine>/   structure only, versioned, run once
db/seeds/<engine>/        data, idempotent, re-run every deploy
```

Migrations never contain INSERTs. Full rationale, the reference-vs-development seed
split, and an honest account of what the per-engine layout does *not* buy you are in
[`db/README.md`](../db/README.md).

Adding a second engine touches `infrastructure` only — the ports in `domain` carry no
SQL, so `domain`, `application` and `api` would not change.

## Notes

- `stats` stays outside the layering: a pure, synchronous supporting subdomain the
  domain layer may eventually call. Keep it free of I/O (see the standing constraints
  in `SETUP.md`).
- sqlx 0.9 accepts only `&'static str` SQL, so queries are `const` literals per query
  rather than composed from a shared column list.
- The old `db` crate is gone; its pool lives in `infrastructure::database` and its
  queries in `infrastructure::persistence`.
