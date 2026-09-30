# Running the API locally

Day-to-day commands for the development loop on WSL Ubuntu.

All commands run from the workspace root: `~/estimator-9-api-rust`.

---

## Quick start

```bash
cd ~/estimator-9-api-rust
cargo run -p api
```

Serves on **http://localhost:3000**. Stop with `Ctrl+C`.

```bash
curl localhost:3000/health     # {"status":"ok","version":"0.1.0"}
```

---

## Auto-reload while developing

```bash
cargo watch -x 'run -p api'
```

Rebuilds and restarts on every `.rs` save — the closest equivalent to PHP's
save-and-refresh. First build takes a couple of minutes; incremental rebuilds after an
edit to `crates/api` are around 6 seconds.

**It does not watch `.env`.** Environment variables are read once at startup, so after
editing `.env` you must stop the watcher and start it again.

---

## Endpoints

| Method | Path | Returns |
|---|---|---|
| GET | `/health` | `{"status":"ok","version":"0.1.0"}` |
| GET | `/users` | all users as JSON |
| GET | `/users/{id}` | one user, or 404 |
| GET | `/swagger-ui` | interactive API docs |
| GET | `/api-docs/openapi.json` | the OpenAPI 3.1 spec |

```bash
curl localhost:3000/health
curl localhost:3000/users
curl localhost:3000/users/1
```

Open **http://localhost:3000/swagger-ui** in a browser to browse and call the
endpoints. The spec is generated from the handler annotations at startup, so it can
never describe a route that isn't mounted.

To write the spec to a file without starting the server or touching the database:

```bash
./scripts/openapi.sh
```

---

## Database

```bash
./scripts/migrate.sh                              # apply pending migrations
./scripts/migrate.sh info                         # applied vs pending
./scripts/migrate.sh revert                       # roll back the most recent

cargo run -p infrastructure --bin seed            # reference data (roles, permissions)
cargo run -p infrastructure --bin seed -- --dev   # + dev-only superadmin bootstrap
```

Migrations hold **structure only**; data lives in `db/seeds/`. Both are grouped per
engine and chosen from the `DATABASE_URL` scheme — see [`db/README.md`](../db/README.md).

Seeds are idempotent and untracked, so re-run them whenever. Migrations are versioned
and run once; editing one after it has been applied fails the checksum check.

The API binds `0.0.0.0`, so these also work from a Windows browser at
`http://localhost:3000`.

---

## Configuration

Set in [`.env`](../.env) at the workspace root. A real shell variable always beats
`.env`, so anything can be overridden for a single run.

| Variable | Default | Purpose |
|---|---|---|
| `DATABASE_URL` | — | required; must be a `postgres://` URL |
| `DB_MAX_CONNECTIONS` | `10` | pool ceiling; sets throughput under load |
| `PORT` | `3000` | HTTP port |
| `RUST_LOG` | — | log filter |
| `ENABLE_DOCS` | `true` | mount `/swagger-ui` and `/api-docs/openapi.json` |

Each variable is read and validated once at startup, in
[`crates/api/src/config/`](../crates/api/src/config/) — one module per concern. A value
that is present but unparseable stops the process with a message naming the variable;
it never falls back to the default silently. Error messages deliberately never include
the *value*, since `DATABASE_URL` carries a password and startup errors end up in
journals and CI logs.

`ENABLE_DOCS` is off only for `false`, `0`, `no`, or `off` — a typo leaves docs on
rather than silently removing them. Both paths return 404 when disabled; the API
itself is unchanged either way.

```bash
PORT=8080 cargo run -p api                          # one-off port change
RUST_LOG=info,tower_http::trace=debug cargo run -p api   # one-off request logging
```

### Request logging on/off

Controlled entirely by `RUST_LOG` — there is no separate flag.

| Value | Effect |
|---|---|
| `info,tower_http::trace=debug,sqlx=warn` | logs method, path, status, latency per request |
| `info,tower_http=off,sqlx=warn` | startup and errors only |
| `…,sqlx=debug` | adds every SQL query |

---

## When the database stops connecting

PostgreSQL runs on the **Windows** side, and WSL2 reassigns its gateway IP across
restarts. When that happens, `DATABASE_URL` points at an IP that no longer answers.

```bash
./scripts/update-db-host.sh
```

Rewrites only the host in `.env`, preserving the password, port, and database name.
Restart the server afterwards.

---

## Common problems

| Symptom | Cause | Fix |
|---|---|---|
| `Address already in use (os error 98)` | another instance still has port 3000 | `pkill -f "target/debug/api"`, or `PORT=8080 cargo run -p api` |
| `Error: environment variable not found` | no `DATABASE_URL` — usually launched from the wrong directory | run from the workspace root so `.env` is found |
| connection timeout to the database | WSL gateway IP changed | `./scripts/update-db-host.sh` |
| `cargo: command not found` | `~/.bashrc` sources `~/.cargo/env` *below* its non-interactive guard | `source "$HOME/.cargo/env"`, or move that line above the guard |
| VS Code "Run" button: *Path to shell executable "cargo" does not exist* | same PATH problem, in VS Code's task runner | launch VS Code with `code .` from a WSL terminal |

---

## Other useful commands

```bash
cargo check -p api          # type-check only, faster than building
cargo clippy --workspace    # lints
cargo nextest run           # tests
cargo build --release -p api
```

Running the production-shaped binary directly, without Cargo:

```bash
./target/x86_64-unknown-linux-musl/release/api
```

See [DEPLOYMENT.md](DEPLOYMENT.md) for how that binary is built and shipped.
