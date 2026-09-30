# Estimator9 API — Staging Deployment

Deploying the `api` binary to a plain, unmanaged Ubuntu server.

**Shape of this deployment:**

| | |
|---|---|
| Build | On your workstation, static musl binary. No toolchain or source on the server. |
| Database | PostgreSQL on the same server, listening on localhost only. Never exposed. |
| Exposure | Nginx on 80/443 with a Let's Encrypt cert; the API itself is not public. |
| Process management | systemd — restarts on failure, starts on boot. |
| Migrations | Run from your workstation over an SSH tunnel. |

Run each step manually. Each has a checkpoint; don't move on until it passes.

This deployment: **estimator9-v2-api.fineit.io** (proxied through Cloudflare), origin **178.105.240.77**.
`STRONG_PASSWORD` stays a placeholder — the real credential lives only in
`/etc/estimator9/api.env` on the server, never in this repo.

---

## Part A — On your workstation

### A1. One-time: musl build toolchain

```bash
sudo apt install -y musl-tools
rustup target add x86_64-unknown-linux-musl
```

`musl-tools` provides `musl-gcc`, which `ring` (the TLS backend) needs to compile its
C sources. Without it the build fails at `ring`'s build script, not at link time —
the error names `cc`, which is misleading.

**Checkpoint**

```bash
musl-gcc --version && rustup target list --installed | grep musl
```

---

### A2. Build the release binary

```bash
cd ~/estimator-9-api-rust
RUSTFLAGS="--remap-path-prefix=$HOME=/build --remap-path-prefix=$PWD=/src" \
  cargo build --release --target x86_64-unknown-linux-musl -p api
```

`--remap-path-prefix` keeps your username and home directory out of the shipped
binary. Without it, `/home/<you>/…` appears several hundred times, embedded in panic
location metadata. The flag is passed at build time because Cargo's equivalent profile
option, `trim-paths`, is still nightly-only as of Cargo 1.98.

Stripping is automatic — [Cargo.toml](../Cargo.toml) sets `strip = true` under
`[profile.release]`, so symbol tables are removed at link time (7.2 MB → 5.5 MB,
15,135 symbols → 0).

**Checkpoint** — confirm it is genuinely static. This is the whole point of musl: a
static binary does not care what glibc version the server has.

```bash
file   target/x86_64-unknown-linux-musl/release/api   # expect: ... static-pie linked ...
ldd    target/x86_64-unknown-linux-musl/release/api   # expect: statically linked
```

If `ldd` lists shared libraries, stop — something pulled in a dynamic dependency and
the binary will not be portable.

**What this does not hide.** Crate names and exact versions remain readable
(`anyhow-1.0.104`, `axum-0.8.9`, …), as do your SQL query strings. Anyone holding the
binary can match those versions against published CVEs. Treat a shipped binary as
readable — see *Not ready for production* below.

---

## Part B — Prepare the server

These commands run **as root** on the server.

> The *deploy* is done as root; the *service* is not. B2 creates an unprivileged
> `estimator9` user, and the systemd unit in C2 runs the API as that user. Never
> change `User=estimator9` to root — a web-facing process should not have it.

### B1. Base packages

```bash
apt update
apt install -y postgresql nginx ufw
```

No `build-essential`, no Rust, no `libpq`, no `libssl` — the static binary needs none
of them.

**Checkpoint**

```bash
systemctl is-active postgresql nginx
```

---

### B2. Service user

The API runs as an unprivileged user that owns nothing and cannot log in.

```bash
useradd --system --no-create-home --shell /usr/sbin/nologin estimator9
```

**Checkpoint**

```bash
id estimator9
```

---

### B3. PostgreSQL role and database

```bash
su - postgres -c psql <<'SQL'
CREATE ROLE estimator9 WITH LOGIN PASSWORD 'STRONG_PASSWORD';
CREATE DATABASE estimator9_staging OWNER estimator9;
SQL
```

**Audit log hardening — run this after the first `./scripts/migrate.sh`:**

```bash
su - postgres -c "psql estimator9_staging" <<'SQL'
REVOKE UPDATE, DELETE ON audit_log FROM estimator9;
SQL
```

The application writes every audit row and must never be able to rewrite one. This
lives here rather than in a migration for a reason: in development the API connects as
`postgres`, and **a superuser bypasses grants entirely**, so the same `REVOKE` in a
migration would apply cleanly and then do nothing. It only bites for a non-superuser
role, which is what this deployment uses.

It does not constrain `estimator9`'s owner rights on other tables, and it does not stop
someone with `postgres` access from editing the log — that is a separate control
(restricted psql access, or `pgaudit` if compliance asks for it).

**Do not touch `listen_addresses`.** Ubuntu's PostgreSQL package defaults to
`localhost` only, which is exactly what you want. Leaving it alone is the security
control — there is no firewall rule to get wrong.

`[::1]:5432` alongside `127.0.0.1:5432` is fine — that is the IPv6 loopback.
The failure signs are `0.0.0.0:5432` or `[::]:5432`, which mean Postgres is listening
on the network.

**Checkpoint** — connects locally, and only locally:

```bash
PGPASSWORD='STRONG_PASSWORD' psql -h 127.0.0.1 -U estimator9 -d estimator9_staging -c 'SELECT 1;'
ss -tlnp | grep 5432      # only loopback: 127.0.0.1:5432 and/or [::1]:5432
```

---

### B4. Environment file

Secrets live here, readable only by the service user. This is **not** a copy of your
dev `.env`.

```bash
mkdir -p /etc/estimator9
tee /etc/estimator9/api.env > /dev/null <<'EOF'
DATABASE_URL=postgres://estimator9:STRONG_PASSWORD@127.0.0.1:5432/estimator9_staging
DB_MAX_CONNECTIONS=10

# Loopback only. Everything below assumes nginx is the sole way in.
BIND_ADDRESS=127.0.0.1
PORT=3000

RUST_LOG=info,tower_http=off,sqlx=warn
ENABLE_DOCS=true

# Generate per deployment: openssl rand -base64 48
JWT_SECRET=REPLACE_ME_WITH_A_FRESH_48_BYTE_SECRET
ACCESS_TOKEN_MINUTES=15
REFRESH_TOKEN_DAYS=7
COOKIE_SECURE=true

CORS_ALLOWED_ORIGINS=https://estimator9-v2-app.fineit.io

ENABLE_RATE_LIMIT=true
RATE_LIMIT_LOGIN_PER_MINUTE=5
RATE_LIMIT_DEFAULT_PER_MINUTE=300
RATE_LIMIT_PASSWORD_RESET_PER_HOUR=3

MAIL_LOG_PATH=/var/log/estimator9/mail.log
PASSWORD_RESET_URL=https://estimator9-v2-app.fineit.io/reset-password
PASSWORD_RESET_MINUTES=60
EOF

chown root:estimator9 /etc/estimator9/api.env
chmod 640 /etc/estimator9/api.env
```

`ENABLE_DOCS` mounts Swagger UI at `/swagger-ui` and the spec at
`/api-docs/openapi.json`. It is left on here because this host is a testing
deployment. **Set it to `false` before the first real dataset lands** — the host is
public and unauthenticated, and published docs turn "anyone can read the user table"
into "anyone can read the user table and is handed the map". Changing it is an edit to
this file plus `systemctl restart estimator9-api`; no rebuild, no new artifact.

**`BIND_ADDRESS=127.0.0.1` is load-bearing, not tidiness.** The rate limiter and
the audit trail both take the caller's address from `X-Real-IP`, which nginx sets
to the real peer. That header is only trustworthy while nginx is the sole way in —
bound to `0.0.0.0`, anyone who can reach port 3000 directly can claim any address
they like, and with it an unlimited share of the rate limit and a false entry in
the audit log. The API logs a warning at startup if it is bound to all interfaces.

**`JWT_SECRET` must be generated per deployment.** Anyone holding it can mint a
token for any account, superadmin included. Never reuse a development value.

**`COOKIE_SECURE=true` requires HTTPS**, which this deployment has. Set to false
only for plain-http testing, or the browser silently drops the refresh cookie and
sign-in appears to work until the first refresh.

**`MAIL_LOG_PATH` is a placeholder.** Password reset messages are written to a
file rather than sent; nobody receives a reset link until an SMTP transport is
configured. Create the directory and make it writable by `estimator9`, or reset
requests will log an error and still return 204 — which is by design, and means
the failure is invisible to the caller.

**Percent-encode special characters in the password.** The value is a URL, so
`Pass@word` is written `Pass%40word`. sqlx itself tolerates a literal `@` (it splits
on the last one, verified), but `/` and `#` genuinely break parsing, and other tools
that read this URL — psql's URI form, pgAdmin — are stricter. Encoding is the habit
that keeps it portable.

One systemd quirk worth knowing: `%` is a specifier escape in unit files, but **not**
in `EnvironmentFile` contents. Percent-encoded passwords are safe in this file. If you
ever inline the URL via `Environment=` in the unit instead, you must double it to `%%`.

**Checkpoint**

```bash
su -s /bin/sh estimator9 -c 'cat /etc/estimator9/api.env >/dev/null' \
  && echo "service user can read it"
```

---

## Part C — Upload and run

### C1. What actually gets uploaded

**Exactly one file: the binary.** Everything else is created on the server.

| File | Goes to | Why |
|---|---|---|
| `target/x86_64-unknown-linux-musl/release/api` | `/usr/local/bin/estimator9-api` | The whole application |

**Do not upload:**

| | Why not |
|---|---|
| `.env` | Dev credentials, and points at your WSL gateway IP. B4 replaces it. |
| `db/migrations/` | Applied from your workstation over the tunnel (Part D). |
| `crates/`, `Cargo.toml`, `Cargo.lock` | The server never compiles anything. |
| `target/` (rest of it) | Build artifacts; gigabytes of nothing useful. |
| `docs/`, `.vscode/`, `scripts/` | Workstation tooling. |

Upload it under its version number, so what landed on the server is never ambiguous:

```bash
# from your workstation
VERSION=$(grep -m1 '^version' crates/api/Cargo.toml | cut -d'"' -f2)
scp target/x86_64-unknown-linux-musl/release/api \
    root@178.105.240.77:/tmp/estimator9-api-$VERSION

# on the server (substitute the version you just uploaded)
install -o root -g root -m 755 /tmp/estimator9-api-0.1.0 /usr/local/bin/estimator9-api
rm /tmp/estimator9-api-0.1.0
```

**Checkpoint** — the binary takes no arguments, so run it bare with no environment.
It should fail *on its own terms*:

```bash
env -u DATABASE_URL /usr/local/bin/estimator9-api
# expect: Error: DATABASE_URL is required but not set   (exit 1)
```

That exact message proves the binary loaded and ran native code. An architecture or
linking problem looks different — `cannot execute binary file`, or a confusing
`No such file or directory` on a binary that plainly exists.

---

### C2. systemd unit

```bash
tee /etc/systemd/system/estimator9-api.service > /dev/null <<'EOF'
[Unit]
Description=Estimator9 API
After=network-online.target postgresql.service
Wants=network-online.target

[Service]
Type=exec
User=estimator9
Group=estimator9
EnvironmentFile=/etc/estimator9/api.env
ExecStart=/usr/local/bin/estimator9-api
Restart=on-failure
RestartSec=5s

NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6
MemoryDenyWriteExecute=true
LockPersonality=true

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable --now estimator9-api
```

`ProtectSystem=strict` makes the entire filesystem read-only to the service. The API
writes no files, so this costs nothing. `AF_UNIX` is required despite the DB being
reached over TCP — journald collects the service's stdout over a unix socket.

**Checkpoint**

```bash
systemctl status estimator9-api --no-pager
curl -s localhost:3000/health      # expect: {"status":"ok","version":"0.1.0"}
```

Check that the version matches what you just uploaded. If it reports an older number,
the service is still running the previous binary.

If it fails to start, `journalctl -u estimator9-api -n 50 --no-pager` will show why —
almost always a malformed `DATABASE_URL` or a password mismatch.

---

### C3. Nginx + TLS (Let's Encrypt)

**Prerequisites in Cloudflare DNS:**

1. The `A` record for `estimator9-v2-api.fineit.io` points to `178.105.240.77`.
2. Proxy status is **DNS only** (grey cloud). Certbot's HTTP-01 challenge needs to
   reach *this* server on port 80; a proxied record would answer from Cloudflare
   instead and the challenge would fail.

Port 80 must be open to the internet — not just for issuing the certificate, but for
every renewal thereafter. C4 allows it via `Nginx Full`.

```bash
tee /etc/nginx/sites-available/estimator9-api > /dev/null <<'EOF'
server {
    listen 80;
    server_name estimator9-v2-api.fineit.io;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host              $host;
        proxy_set_header X-Real-IP         $remote_addr;
        proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
EOF

ln -sf /etc/nginx/sites-available/estimator9-api /etc/nginx/sites-enabled/
rm -f /etc/nginx/sites-enabled/default
nginx -t && systemctl reload nginx
```

**Checkpoint** — plain HTTP works before introducing TLS:

```bash
curl -s http://estimator9-v2-api.fineit.io/health
```

If this fails, fix it before running certbot — the challenge uses the same path.

Then issue the certificate. Certbot edits the file above in place, adding a 443 server
block and an HTTP→HTTPS redirect:

```bash
apt install -y certbot python3-certbot-nginx
certbot --nginx -d estimator9-v2-api.fineit.io
```

**Checkpoint**

```bash
curl -s https://estimator9-v2-api.fineit.io/health
# expect: {"status":"ok","version":"0.1.0"}

systemctl list-timers certbot.timer --no-pager   # renewal timer is active
certbot renew --dry-run                          # renewal actually works
```

Certificates last 90 days and the timer renews them automatically. The dry run is
worth doing now rather than discovering a broken renewal in three months.

> **The API is publicly reachable and has no authentication.** Anyone who resolves
> this hostname can read the full user table. That is accepted for the testing phase —
> see *Not ready for production* below.

> If you later switch Cloudflare back to **Proxied**, two things must change: set
> SSL/TLS mode to *Full (strict)*, and change `X-Real-IP` to
> `$http_cf_connecting_ip`, since `$remote_addr` would then be a Cloudflare edge
> rather than the caller.

---

### C4. Firewall

```bash
ufw allow OpenSSH
ufw allow 'Nginx Full'
ufw --force enable
```

**Checkpoint** — on the server:

```bash
ufw status          # 3000 absent from the allow list
```

Then **from your own machine, not the server** — the point is to prove port 3000 is
unreachable from the internet. Run on the server it would hit loopback and succeed,
which proves nothing:

```bash
curl --max-time 5 http://178.105.240.77:3000/health   # must fail / time out
curl -s https://estimator9-v2-api.fineit.io/health    # must succeed
```

The pair is deliberately opposite: the API is reachable only through nginx on 443,
never directly on 3000.

> **Known gap.** The binary currently hardcodes `bind("0.0.0.0:{port}")`
> ([crates/api/src/main.rs](../crates/api/src/main.rs)), so it listens on every interface
> and ufw is the only thing keeping port 3000 private. Making the bind address
> configurable — and setting it to `127.0.0.1` here — would make this defence-in-depth
> rather than a single point of failure. Worth doing before anything sensitive lands.

---

## Part D — Migrations

The server has no Rust toolchain and Postgres accepts no remote connections, so
migrations run **from your workstation through an SSH tunnel**. The tunnel terminates
on the server and connects onward to `127.0.0.1:5432`, so nothing is exposed.

This keeps the `_sqlx_migrations` ledger authoritative. Applying the `.sql` files by
hand with `psql` would leave that ledger empty and break every future
`./scripts/migrate.sh`.

Use the origin **IP**, not the Cloudflare hostname — the free plan proxies HTTP(S)
only, so SSH must reach the server directly.

In one terminal:

```bash
ssh -N -L 5433:127.0.0.1:5432 root@178.105.240.77
```

In another, from the repo root:

```bash
cd ~/estimator-9-api-rust
DATABASE_URL='postgres://estimator9:STRONG_PASSWORD@127.0.0.1:5433/estimator9_staging' \
  ./scripts/migrate.sh
```

**Checkpoint**

```bash
DATABASE_URL='postgres://estimator9:STRONG_PASSWORD@127.0.0.1:5433/estimator9_staging' \
  ./scripts/migrate.sh info
```

Every migration should read `installed`. Then close the tunnel.

Per SETUP.md's migration rules, staging is **forward-only** — never run
`./scripts/migrate.sh revert` against it. Fix forward with a new migration.

---

## Redeploying

**Bump the version first.** Edit `version` in
[crates/api/Cargo.toml](../crates/api/Cargo.toml) — that value is compiled in via
`env!("CARGO_PKG_VERSION")` and is what `/health` reports, so an unbumped version
makes it impossible to tell a successful deploy from a silently failed one.

```bash
# workstation
VERSION=$(grep -m1 '^version' crates/api/Cargo.toml | cut -d'"' -f2)
RUSTFLAGS="--remap-path-prefix=$HOME=/build --remap-path-prefix=$PWD=/src" \
  cargo build --release --target x86_64-unknown-linux-musl -p api
scp target/x86_64-unknown-linux-musl/release/api \
    root@178.105.240.77:/tmp/estimator9-api-$VERSION

# server
cp /usr/local/bin/estimator9-api /usr/local/bin/estimator9-api.prev && \
install -m 755 /tmp/estimator9-api-$VERSION /usr/local/bin/estimator9-api && \
systemctl restart estimator9-api && \
sleep 2 && curl -s localhost:3000/health
```

That last `curl` is the real check — it must report the version you just built.

Run migrations (Part D) **before** restarting if the release contains any.

**Rollback:**

```bash
cp /usr/local/bin/estimator9-api.prev /usr/local/bin/estimator9-api
systemctl restart estimator9-api
```

---

## Operating it

| Task | Command |
|---|---|
| Logs, live | `journalctl -u estimator9-api -f` |
| Logs, last 100 | `journalctl -u estimator9-api -n 100 --no-pager` |
| Restart | `systemctl restart estimator9-api` |
| Status | `systemctl status estimator9-api` |
| Change env/secrets | edit `/etc/estimator9/api.env`, then restart |

`RUST_LOG` in the env file controls verbosity. `tower_http::trace=debug` turns on
per-request logging; `tower_http=off` (the default here) silences it. Changes require
a restart — the value is read once at startup.

---

## Not ready for production

Staging-appropriate, but each of these must be closed before this handles real data:

| Gap | Impact |
|---|---|
| **No authentication on any endpoint** | Anyone who reaches the domain can read the full user table. |
| **Binds `0.0.0.0`** | Only ufw keeps port 3000 private (see C4). |
| **No graceful shutdown** | `tokio`'s `signal` feature is a dependency but unused; SIGTERM drops in-flight requests. |
| **No rate limiting** | Nothing in front of the API but nginx defaults. |
| **No backups** | Nothing dumps `estimator9_staging` anywhere. |

---

## Checkpoint summary

| # | Command | Expected |
|---|---|---|
| A1 | `musl-gcc --version` | version printed |
| A2 | `ldd target/.../release/api` | **statically linked** |
| B1 | `systemctl is-active postgresql nginx` | `active` twice |
| B3 | `ss -tlnp \| grep 5432` | `127.0.0.1:5432` only |
| C1 | `env -u DATABASE_URL /usr/local/bin/estimator9-api` | `Error: DATABASE_URL is required but not set` |
| C2 | `curl -s localhost:3000/health` | `{"status":"ok","version":"0.1.0"}` |
| C3 | `curl -s https://estimator9-v2-api.fineit.io/health` | version matches what you built |
| C4 | `curl http://178.105.240.77:3000/health` *(from your machine)* | fails / times out |
| D |  `./scripts/migrate.sh info` | all `installed` |
