#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

SCENARIOS=(signup-session login-access profile-update graph-update image-upload)
SCENARIO="${1:-all}"
if [[ "$#" -gt 1 ]]; then
  echo "Usage: $0 [${SCENARIOS[*]}]" >&2
  exit 2
fi
# shellcheck disable=SC2076
if [[ "$SCENARIO" != all ]] && [[ ! " ${SCENARIOS[*]} " =~ " ${SCENARIO} " ]]; then
  echo "Unknown integration scenario: $SCENARIO" >&2
  echo "Usage: $0 [${SCENARIOS[*]}]" >&2
  exit 2
fi

for command in docker cargo curl npm python3; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "Required command not found: $command" >&2
    exit 1
  fi
done

docker compose version >/dev/null

PROJECT="rusty-integration-$$-${RANDOM}"
COMPOSE=(docker compose -p "$PROJECT" -f compose.yaml -f compose.integration.yaml)
export COMPOSE_PROJECT_NAME="$PROJECT"
APP_PID=""
mkdir -p "$ROOT/target"
TEST_TMP_DIR="$(mktemp -d "$ROOT/target/integration-tests.XXXXXX")"
export TEST_TMP_DIR

export POSTGRES_USER=rusty_test
export POSTGRES_PASSWORD=integration_test_only
export POSTGRES_DB=rusty_test
export S3_ACCESS_KEY=rusty_integration
export S3_SECRET_KEY=integration_test_only
export SQLX_OFFLINE=true

if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
  PASS_STYLE=$'\033[32m'
  SECTION_STYLE=$'\033[1m'
  RESET_STYLE=$'\033[0m'
else
  PASS_STYLE=""
  SECTION_STYLE=""
  RESET_STYLE=""
fi

report_pass() {
  printf '%s✓ PASS%s %s\n' "$PASS_STYLE" "$RESET_STYLE" "$1"
}

report_section() {
  printf '\n%s== %s ==%s\n' "$SECTION_STYLE" "$1" "$RESET_STYLE"
}

cleanup() {
  local exit_code=$?
  trap - EXIT INT TERM

  if [[ -n "$APP_PID" ]] && kill -0 "$APP_PID" 2>/dev/null; then
    kill "$APP_PID" 2>/dev/null || true
    wait "$APP_PID" 2>/dev/null || true
  fi

  if [[ "$exit_code" -ne 0 && -f "$TEST_TMP_DIR/app.log" ]]; then
    echo "Application log (last 100 lines):" >&2
    tail -n 100 "$TEST_TMP_DIR/app.log" >&2
  fi

  echo "Removing isolated integration-test services and volumes..."
  "${COMPOSE[@]}" down --volumes --remove-orphans || true
  rm -rf -- "$TEST_TMP_DIR"
  exit "$exit_code"
}

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

report_section "JavaScript and Rust unit and regression tests"
npm test

report_section "Starting isolated PostgreSQL, Redis, and S3-compatible services"
"${COMPOSE[@]}" up -d --wait --wait-timeout 180 postgres redis storage

published_port() {
  "${COMPOSE[@]}" port "$1" "$2" | sed 's/.*://'
}

POSTGRES_PORT="$(published_port postgres 5432)"
REDIS_PORT="$(published_port redis 6379)"
S3_PORT="$(published_port storage 9000)"
APP_PORT="$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')"

export APP_ENV=development
export APP_HOST=127.0.0.1
export APP_PORT
export BASE_URL="http://${APP_HOST}:${APP_PORT}"
export DATABASE_URL="postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@127.0.0.1:${POSTGRES_PORT}/${POSTGRES_DB}"
export CACHE_URL="redis://127.0.0.1:${REDIS_PORT}"
export S3_ENDPOINT="http://127.0.0.1:${S3_PORT}"
export S3_BUCKET=rusty-integration-test
export RUST_LOG=info
export RATE_LIMIT_HMAC_KEY=integration-tests-only-hmac-key-do-not-use-in-production

report_section "Building the application"
cargo build

report_section "Starting the application against the isolated services"
./target/debug/rusty-noname >"$TEST_TMP_DIR/app.log" 2>&1 &
APP_PID=$!
ready=0

for _ in {1..60}; do
  if ! kill -0 "$APP_PID" 2>/dev/null; then
    echo "Application exited before becoming ready." >&2
    exit 1
  fi

  if curl --fail --silent --max-time 2 "${BASE_URL}/login" >/dev/null 2>&1; then
    ready=1
    break
  fi

  sleep 1
done

if [[ "$ready" != 1 ]]; then
  echo "Application did not become ready at ${BASE_URL}/login." >&2
  exit 1
fi
report_pass "Application /login endpoint is responding"

migration_count="$("${COMPOSE[@]}" exec -T postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -Atqc 'SELECT COUNT(*) FROM _sqlx_migrations')"
if [[ "$migration_count" -lt 1 ]]; then
  echo "Expected SQLx migrations to be applied, found $migration_count." >&2
  exit 1
fi
report_pass "PostgreSQL applied $migration_count migration(s)"

redis_response="$("${COMPOSE[@]}" exec -T redis redis-cli ping)"
if [[ "$redis_response" != PONG ]]; then
  echo "Redis health check returned: $redis_response" >&2
  exit 1
fi
report_pass "Redis responds to PING"

image_content_type="$(curl --fail --silent --show-error --max-time 10 -o /dev/null -w '%{content_type}' "${BASE_URL}/images/defaults/default_logo.webp")"
if [[ "$image_content_type" != "image/webp" ]]; then
  echo "Expected the default image endpoint to return image/webp, got: $image_content_type" >&2
  exit 1
fi
report_pass "S3-backed default image endpoint returned image/webp"

run_scenario() {
  local scenario="$1"
  report_section "Scenario: $scenario"
  bash "$ROOT/scripts/integration-tests/${scenario}.sh"
  report_pass "Scenario completed: $scenario"
}

if [[ "$SCENARIO" == all ]]; then
  for scenario in "${SCENARIOS[@]}"; do
    run_scenario "$scenario"
  done
else
  run_scenario "$SCENARIO"
fi

report_pass "All integration tests passed"