#!/usr/bin/env bash
# Rewrites DATABASE_URL's host in .env to the current WSL2 NAT gateway IP,
# since that IP changes across WSL restarts (Postgres runs on the Windows side).
set -euo pipefail

cd "$(dirname "$0")/.."

HOST=$(awk '/^nameserver/{print $2; exit}' /etc/resolv.conf)

if [[ -z "$HOST" ]]; then
  echo "Could not read nameserver IP from /etc/resolv.conf" >&2
  exit 1
fi

if [[ -f .env ]] && grep -q '^DATABASE_URL=postgres://' .env; then
  sed -i -E "s#(^DATABASE_URL=postgres://[^@]+@)[^:/]+(:.*)#\1${HOST}\2#" .env
else
  echo "DATABASE_URL=postgres://USER:PASS@${HOST}:5432/estimator9_dev" >> .env
  echo "Created .env with a placeholder — edit USER:PASS before running anything." >&2
fi

echo "DATABASE_URL host set to ${HOST}"
