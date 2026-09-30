#!/usr/bin/env bash
# Wraps sqlx-cli so the per-engine migration folder is derived from DATABASE_URL
# rather than retyped as --source on every call.
#
#   ./scripts/migrate.sh            apply pending migrations
#   ./scripts/migrate.sh info       what is applied, what is pending
#   ./scripts/migrate.sh revert     roll back the most recent migration
#   ./scripts/migrate.sh add <name> new migration pair in the right folder
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "DATABASE_URL is not set (checked the environment and .env)" >&2
  exit 1
fi

case "${DATABASE_URL%%:*}" in
  postgres | postgresql) ENGINE=postgres ;;
  mysql | mariadb)       ENGINE=mysql ;;
  oracle)                ENGINE=oracle ;;
  *) echo "Cannot tell the engine from DATABASE_URL" >&2; exit 1 ;;
esac

SOURCE="db/migrations/${ENGINE}"

if [[ ! -d "$SOURCE" ]]; then
  echo "No migrations folder at ${SOURCE}" >&2
  exit 1
fi

COMMAND=${1:-run}
shift || true

echo "engine: ${ENGINE}  source: ${SOURCE}"
exec sqlx migrate "$COMMAND" --source "$SOURCE" "$@"
