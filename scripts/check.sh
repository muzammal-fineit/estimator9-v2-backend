#!/usr/bin/env bash
# Lint, test, and check the OpenAPI spec, layer boundaries and file sizes.
# Prints a short report designed to be pasted back into a chat rather than read
# in full.
#
#   ./scripts/check.sh              everything
#   ./scripts/check.sh --code       fmt, clippy, tests, openapi, layers, filesize
#   ./scripts/check.sh --endpoints  only the live endpoint checks
#   ./scripts/check.sh --fix        regenerate docs/openapi.json instead of
#                                   failing when it is stale
#
# --code needs no database and no running server. --endpoints compiles nothing
# and needs a server already answering on $PORT, so it is instant.
set -uo pipefail

cd "$(dirname "$0")/.."

RUN_CODE=1
RUN_ENDPOINTS=1
FIX=0

for arg in "$@"; do
  case "$arg" in
    --code|-c)      RUN_ENDPOINTS=0 ;;
    --endpoints|-e) RUN_CODE=0 ;;
    --all|-a)       ;;
    --fix|-f)       FIX=1 ;;
    -h|--help)
      sed -n '2,12p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *)
      echo "unknown option: $arg (try --help)" >&2
      exit 2
      ;;
  esac
done

PORT="${PORT:-3000}"
BASE="http://localhost:${PORT}"
LOG=$(mktemp -d)
trap 'rm -rf "$LOG"' EXIT

FAILED=0
STARTED=$(date +%s%N)

# Seconds elapsed since the nanosecond timestamp in $1.
since() {
  awk -v ns="$(( $(date +%s%N) - $1 ))" 'BEGIN { printf "%6.1fs", ns / 1000000000 }'
}

# Runs a step, prints one timed line, and on failure the tail of its output.
step() {
  local name=$1 lines=$2
  shift 2
  local began
  began=$(date +%s%N)

  if "$@" > "$LOG/$name.txt" 2>&1; then
    printf '%-10s ok     %s\n' "$name" "$(since "$began")"
    return 0
  fi

  printf '%-10s FAILED %s\n' "$name" "$(since "$began")"
  sed 's/^/           | /' <(tail -n "$lines" "$LOG/$name.txt")
  FAILED=1
  return 1
}

echo "--- estimator9 check ---"

if [[ $RUN_CODE -eq 1 ]]; then
  # rust-analyzer is disabled for this workspace, so nothing formats on save.
  # Compiles nothing, so it runs first. Fix with: cargo fmt --all
  step fmt 20 cargo fmt --all --check

  # Clippy type-checks every target that `cargo build` would, but under its own
  # fingerprint — running both would compile the whole workspace twice to learn
  # the same thing. Linking is covered by the `binaries` step below.
  step clippy 30 cargo clippy --workspace --all-targets -- -D warnings

  # nextest runs tests in parallel. That matters here because Argon2 is
  # deliberately slow, so a handful of hashes dominate the suite's wall clock.
  if command -v cargo-nextest > /dev/null 2>&1; then
    TEST=(cargo nextest run --workspace --status-level fail)
  else
    TEST=(cargo test --workspace)
  fi

  if step tests 20 "${TEST[@]}"; then
    grep -hE "tests run:|^test result:" "$LOG/tests.txt" | tail -1 \
      | sed 's/^[[:space:]]*/           /'
  fi

  # Clippy type-checks every target but links none of them, and `seed` and
  # `setpassword` have no tests and are never run here — so a link error in
  # either would otherwise stay hidden until someone invoked them.
  step binaries 20 cargo build --workspace --bins

  # The spec is generated from the handler annotations, so a stale committed copy
  # means someone changed an endpoint without regenerating it.
  OPENAPI_BEGAN=$(date +%s%N)
  if cargo run -q -p api --bin openapi > "$LOG/openapi.json" 2>"$LOG/openapi.err"; then
    if diff -q docs/openapi.json "$LOG/openapi.json" > /dev/null 2>&1; then
      printf '%-10s ok     %s (docs/openapi.json current)\n' "openapi" "$(since "$OPENAPI_BEGAN")"
    elif [[ $FIX -eq 1 ]]; then
      # Asked for explicitly. The committed spec is the API's published
      # contract, so overwriting it is a decision, not a default.
      cp "$LOG/openapi.json" docs/openapi.json
      printf '%-10s regenerated (docs/openapi.json updated)\n' "openapi"
    else
      printf '%-10s STALE — review the diff, then: ./scripts/openapi.sh\n' "openapi"
      printf '           %s\n' "(or re-run with --fix to accept it)"
      diff <(jq -S . docs/openapi.json 2>/dev/null || cat docs/openapi.json) \
           <(jq -S . "$LOG/openapi.json" 2>/dev/null || cat "$LOG/openapi.json") \
        | head -20 | sed 's/^/           | /'
      FAILED=1
    fi
  else
    printf '%-10s FAILED\n' "openapi"
    tail -10 "$LOG/openapi.err" | sed 's/^/           | /'
    FAILED=1
  fi

  # The dependency rule, checked rather than trusted: no web or SQL framework may
  # appear below the api crate.
  LEAKS=$(grep -rln "utoipa\|ToSchema\|axum\|sqlx" \
            crates/domain/src crates/application/src 2>/dev/null)
  if [[ -z "$LEAKS" ]]; then
    printf '%-10s ok (domain + application are framework-free)\n' "layers"
  else
    printf '%-10s LEAK\n' "layers"
    sed 's/^/           | /' <<< "$LEAKS"
    FAILED=1
  fi

  # No fat files. A source file over the limit has usually absorbed a second
  # responsibility that deserves its own file — see "One thing per file" in
  # docs/ARCHITECTURE.md before splitting one just to satisfy the counter.
  MAX_LINES=150
  FAT=$(find crates -name '*.rs' -not -path '*/.git/*' \
          -exec awk -v m="$MAX_LINES" 'END { if (NR > m) printf "%s — %d lines\n", FILENAME, NR }' {} \;)

  if [[ -z "$FAT" ]]; then
    LARGEST=$(find crates -name '*.rs' -not -path '*/.git/*' -exec wc -l {} + \
                | sort -rn | grep -v ' total$' | head -1 | awk '{ printf "%s at %d", $2, $1 }')
    printf '%-10s ok (limit %d; largest is %s)\n' "filesize" "$MAX_LINES" "$LARGEST"
  else
    printf '%-10s OVER %d LINES — split these:\n' "filesize" "$MAX_LINES"
    sed 's/^/           | /' <<< "$FAT"
    FAILED=1
  fi
fi

if [[ $RUN_ENDPOINTS -eq 1 ]]; then
  # Only meaningful if a server happens to be running; never starts one.
  if curl -sf -o /dev/null --max-time 2 "$BASE/health" 2>/dev/null; then
    echo "--- endpoints (server on :$PORT) ---"
    for path in /health /users /users/1 /users/999999 /swagger-ui /api-docs/openapi.json; do
      printf '%-24s %s\n' "$path" \
        "$(curl -sL -o /dev/null -w '%{http_code}' --max-time 5 "$BASE$path")"
    done
  else
    echo "endpoints  skipped (no server on :$PORT — start one with: cargo run -p api)"
  fi
fi

echo "--- $([[ $FAILED -eq 0 ]] && echo "all green" || echo "SOMETHING FAILED") in$(since "$STARTED") ---"
exit $FAILED
