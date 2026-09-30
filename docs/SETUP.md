# Estimator9 API (Rust) — Environment Setup

Setup for the Rust rewrite, scoped to what the **MEV API slice** needs.
Engine binary, queue, and Excel I/O come later.

**Environment:** WSL Ubuntu, PostgreSQL already running locally.
**Workspace root:** `~/estimator-9-api-rust` (this folder — the workspace is built in place).

Run each step manually. Each has a checkpoint; don't move on until it passes.

---

## 1. System prerequisites

```bash
sudo apt update
sudo apt install -y build-essential pkg-config curl git
```

`build-essential` provides the system linker that Rust needs.

**Do NOT install `libssl-dev`.** The stack uses `rustls`. Installing OpenSSL headers
invites a crate to link against them, which silently breaks the static build later.

**Checkpoint**

```bash
cc --version
```

---

## 2. Rust toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup default stable
rustup component add rustfmt clippy
```

**Checkpoint**

```bash
rustc --version && cargo --version && cargo clippy --version
```

---

## 3. Dev tooling

```bash
cargo install cargo-nextest --locked
cargo install cargo-insta   --locked
cargo install sqlx-cli --no-default-features --features postgres,rustls --locked
cargo install cargo-watch   --locked   # optional: rebuild on save
```

**Faster linking — required, not optional:**

```bash
sudo apt install -y mold
mold --version
```

`.cargo/config.toml` points the dev build at mold, so **without it every build
fails at link time**. Linking is what dominates an incremental rebuild; the compile
is cached, the link is not.

It applies to the `x86_64-unknown-linux-gnu` dev target only. The static musl
release build stays on the stock linker, so the artifact you actually ship is not
affected by a workstation tool.

These compile from source — a few minutes.

| Tool | Why |
|---|---|
| `cargo-nextest` | Faster parallel test runner. Iteration speed is the main cost of choosing Rust. |
| `cargo-insta` | Snapshot testing — the tool for golden-dataset parity fixtures. |
| `sqlx-cli` | Migration runner + offline query metadata. |

**Checkpoint**

```bash
cargo nextest --version && sqlx --version && cargo insta --version
```

---

## 4. Workspace scaffold

From inside `~/estimator-9-api-rust`:

```bash
cd ~/estimator-9-api-rust
git init
mkdir -p crates db/migrations/postgres
cargo new --lib crates/stats
cargo new --lib crates/db
cargo new --bin crates/api
```

Create the workspace manifest at the root:

```bash
cat > Cargo.toml <<'EOF'
[workspace]
resolver = "2"
members = ["crates/stats", "crates/db", "crates/api"]

[workspace.package]
edition = "2021"

[workspace.dependencies]
serde        = { version = "1", features = ["derive"] }
serde_json   = "1"
thiserror    = "2"
anyhow       = "1"
chrono       = { version = "0.4", features = ["serde"] }
rust_decimal = { version = "1", features = ["serde"] }
tracing      = "0.1"
EOF
```

**Why three crates:** small crates keep incremental rebuilds narrow, which directly
mitigates Rust's compile-time cost over a long parity effort.

| Crate | Role | Async? |
|---|---|---|
| `stats` | ADF, OLS/QR, MEV regression. Pure, no I/O. | No |
| `db` | Queries, types, migrations | Yes (sqlx) |
| `api` | axum HTTP layer | Yes |

> **Superseded.** The workspace has since been split along DDD layers — `db` was
> replaced by `domain`, `application`, and `infrastructure`. The reasoning above still
> holds; there are just five crates now. See `ARCHITECTURE.md` for the current layout
> and the dependency rule. The steps below remain the record of the original setup.

**Checkpoint**

```bash
cargo build
cargo metadata --no-deps --format-version 1 | head -c 400
```

---

## 5. Crate dependencies

### 5a. `crates/stats` — pure numerics, synchronous

```bash
cd ~/estimator-9-api-rust/crates/stats
cargo add nalgebra statrs
cargo add serde --features derive
cargo add thiserror
cargo add --dev insta --features json
cargo add --dev rstest approx
```

`nalgebra` for QR and SVD. `statrs` for distributions **only** — not for inference.

### 5b. `crates/db`

```bash
cd ~/estimator-9-api-rust/crates/db
cargo add sqlx --no-default-features \
  --features postgres,runtime-tokio,tls-rustls-ring,macros,chrono,rust_decimal,json,migrate
cargo add serde --features derive
cargo add serde_json chrono thiserror
cargo add rust_decimal
```

**Do not add `rust_decimal`'s `db-postgres` feature here.** That feature wires
`rust_decimal` into the **`postgres` crate** (rust-postgres, synchronous). This crate
uses **sqlx**, which has its own `rust_decimal` feature — already enabled above.
Adding `db-postgres` does nothing useful and breaks the workspace-inherited feature set.

It becomes correct later, when the synchronous engine crate uses `postgres` directly.

Feature notes (sqlx 0.9 — these changed from older guides):

- `runtime-tokio-rustls` no longer exists. Runtime and TLS are now chosen
  separately: `runtime-tokio` + a `tls-*` feature.
- Use **`tls-rustls-ring`**, not plain `tls-rustls`. The generic alias resolves to the
  `aws-lc-rs` backend, which requires **cmake and a C toolchain** — a native build
  dependency that complicates the static musl build. `ring` avoids it.
- `--no-default-features` is still required; defaults enable `any` plus extras
  we don't want.
- sqlx 0.9 requires Rust **1.94+**.

### 5c. `crates/api`

```bash
cd ~/estimator-9-api-rust/crates/api
cargo add axum
cargo add tokio --features rt-multi-thread,macros,signal
cargo add tower-http --features trace,cors
cargo add serde --features derive
cargo add serde_json chrono anyhow thiserror
cargo add uuid --features v4
cargo add tracing
cargo add tracing-subscriber --features env-filter
cargo add dotenvy
```

Note: `--features` applies to a **single** crate. `cargo add a b --features x` fails,
because cargo can't tell which crate `x` belongs to — add those separately.

**Checkpoint** — build, then confirm OpenSSL never entered the tree:

```bash
cd ~/estimator-9-api-rust
cargo build
cargo tree -i openssl-sys
```

Also confirm the TLS backend is `ring`, not `aws-lc-rs`:

```bash
cargo tree -i aws-lc-rs    # must report: no matching package
cargo tree -i ring         # must show: ring -> rustls -> sqlx-core
```

`openssl-sys` and `aws-lc-rs` must both report **no matching package**.
If either appears, stop and trace it — far cheaper to fix now than at packaging time,
when the static musl build would fail.

---

## 6. Database wiring

```bash
cd ~/estimator-9-api-rust

cat > .env <<'EOF'
DATABASE_URL=postgres://USER:PASS@localhost:5432/estimator9_dev
RUST_LOG=info,sqlx=warn
EOF

cat > .gitignore <<'EOF'
target/
.env
EOF
```

Edit `.env` with real credentials, then create the database:

```bash
createdb estimator9_dev     # or via your usual Postgres tooling
```

Migrations come with the first code slice:

```bash
./scripts/migrate.sh add -r 0001_mev_schema   # pair in db/migrations/<engine>/
# write the SQL by hand — never generate it from a DSL
./scripts/migrate.sh
cargo sqlx prepare --workspace        # writes .sqlx/ for offline compile-time checks
```

**Migration rules** (from the architecture plan):

1. Hand-written SQL is the single source of truth.
2. Idempotent — `IF NOT EXISTS`, guarded `ALTER`. On-prem installers get re-run.
3. Forward-only in production. No `DROP COLUMN` in the same release as its
   replacement: add → backfill → switch → drop later.
4. Explicit transactions, except `CREATE INDEX CONCURRENTLY` (own file).
5. Commit `.sqlx/`. CI runs with `SQLX_OFFLINE=true`.
6. **Structure only — no INSERTs.** Data lives in `db/seeds/`; see `db/README.md`.

Migrations and seeds are grouped per engine under `db/`, and `scripts/migrate.sh`
picks the folder from `DATABASE_URL` so the `--source` flag is never retyped.

---

## 7. Static build target (optional now)

```bash
rustup target add x86_64-unknown-linux-musl
sudo apt install -y musl-tools
```

Not needed for development. Worth verifying early anyway — discovering a crate that
won't build against musl is much cheaper to learn now than at packaging time.

```bash
cargo build --target x86_64-unknown-linux-musl
```

---

## IDE — VS Code

**Open the folder from inside WSL** (`code .` in the WSL terminal, or "Reopen in WSL"
from the command palette). Confirm the bottom-left indicator reads `WSL: Ubuntu`.

Opening `\\wsl.localhost\...` from Windows instead runs rust-analyzer as a Windows
process against a network share — slow, and it won't see the Linux toolchain.

Extensions:

| Extension | ID | Notes |
|---|---|---|
| Remote - WSL | `ms-vscode-remote.remote-wsl` | Required — see above |
| rust-analyzer | `rust-lang.rust-analyzer` | The language server. Heavy: 1–3 GB RAM. |
| CodeLLDB | `vadimcn.vscode-lldb` | Debugger |
| Even Better TOML | `tamasfe.even-better-toml` | Cargo.toml schema + validation |
| Dependi | `fill-labs.dependi` | Crate version / advisory hints |
| Error Lens | `usernamehw.errorlens` | Inline diagnostics (optional) |
| PostgreSQL | `ms-ossdata.vscode-pgsql` | Query the dev DB (optional) |

**Remove `rust-lang.rust` if present** — deprecated, conflicts with rust-analyzer.

Workspace settings are committed at `.vscode/settings.json`. The important one is
`rust-analyzer.cargo.targetDir: true` — without it, rust-analyzer and terminal
`cargo build` contend for the `target/` lock and block each other.

**WSL memory:** rust-analyzer runs inside WSL, which caps its own allocation. If it
gets OOM-killed it fails silently — completions just stop. On a 16 GB machine, create
`C:\Users\<you>\.wslconfig`:

```ini
[wsl2]
memory=8GB
```

Then `wsl --shutdown` and reopen.

---

## Checkpoint summary

| # | Command | Expected |
|---|---|---|
| 1 | `cc --version` | gcc version printed |
| 2 | `rustc --version` | stable toolchain |
| 3 | `cargo nextest --version` | version printed |
| 4 | `cargo build` | empty workspace builds |
| 5 | `cargo tree -i openssl-sys` | **no matching package** |
| 5b | `cargo tree -i aws-lc-rs` | **no matching package** |
| 5c | `cargo tree -i ring` | `ring → rustls → sqlx-core` |
| 6 | `./scripts/migrate.sh` | applies cleanly |

---

## Standing constraints

Carried from the architecture decisions — these hold for all code in this repo.

- **`rust_decimal` for every monetary value.** `f64` only for statistical
  intermediates that never reach a provision figure.
- **QR or SVD for regression. Never normal equations.** MEV sets (GDP, unemployment,
  inflation, policy rate, FX) are collinear by construction — exactly where
  normal-equations OLS silently produces wrong coefficients.
- **No BLAS backend** (`ndarray-linalg` + OpenBLAS/MKL) unless a benchmark forces it.
  It reintroduces shipped native libraries and makes reduction order depend on thread
  count, which breaks determinism.
- **`rustls`, never `native-tls`/OpenSSL.** Breaks static linking.
- **Determinism test in week one** — one portfolio, 50 runs, assert bit-identical
  output. Compare `Decimal` with `==`; for any `f64` that reaches output, compare
  `.to_bits()`.
- **Engine will be synchronous** (no Tokio). Async is confined to this API crate.
- **Postgres does the set-based work** — streaming reads, `COPY` writes, jsonb
  extraction in the projection, date casting in SQL. Use `numeric` for monetary
  aggregates in SQL, never `double precision` (parallel query reorders float sums).

---

## Next

Once all six checkpoints pass, the first slice is:

1. `0001_mev_schema` migration — MEV series, observations, models, coefficients
2. `crates/stats` — OLS via QR, with NIST StRD validation tests (Longley, Filip)
3. `crates/api` — MEV CRUD + fit endpoints

**Open decision:** whether MEV regression fitting runs synchronously in the request
(reasonable at 40–120 observations, and much simpler) or goes through the job queue.
