#!/usr/bin/env bash
# Regenerate docs/openapi.json from the handler annotations.
#
# Writes to a temporary file and moves it into place only on success. Doing
# `cargo run ... > docs/openapi.json` directly is a trap: the shell truncates
# the target *before* running the command, so a build failure leaves you with
# an empty committed spec on top of whatever you were already fixing.
set -euo pipefail

cd "$(dirname "$0")/.."

TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT

cargo run -q -p api --bin openapi > "$TMP"

# Guard against a binary that exits 0 having written nothing useful.
if [[ ! -s "$TMP" ]]; then
  echo "the openapi binary produced no output; docs/openapi.json left unchanged" >&2
  exit 1
fi

if cmp -s "$TMP" docs/openapi.json; then
  echo "docs/openapi.json already current"
else
  mv "$TMP" docs/openapi.json
  echo "docs/openapi.json updated"
fi
