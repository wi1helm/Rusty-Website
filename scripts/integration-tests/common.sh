#!/usr/bin/env bash
set -euo pipefail

INTEGRATION_TESTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$INTEGRATION_TESTS_DIR/../.." && pwd)"

if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
  PASS_STYLE=$'\033[32m'
  FAIL_STYLE=$'\033[31m'
  RESET_STYLE=$'\033[0m'
else
  PASS_STYLE=""
  FAIL_STYLE=""
  RESET_STYLE=""
fi

pass() {
  printf '  %s✓ PASS%s %s\n' "$PASS_STYLE" "$RESET_STYLE" "$1"
}

fail() {
  printf '%s✗ FAIL%s Integration test failed: %s\n' "$FAIL_STYLE" "$RESET_STYLE" "$*" >&2
  exit 1
}

if [[ -z "${BASE_URL:-}" ]]; then
  fail "BASE_URL must point to a running test application."
fi

if [[ -z "${TEST_TMP_DIR:-}" ]]; then
  mkdir -p "$ROOT/target"
  TEST_TMP_DIR="$(mktemp -d "$ROOT/target/integration-tests.XXXXXX")"
  trap 'rm -rf -- "$TEST_TMP_DIR"' EXIT
fi

http_request() {
  local request_name="$1"
  shift

  RESPONSE_HEADERS="$TEST_TMP_DIR/${request_name}.headers"
  RESPONSE_BODY="$TEST_TMP_DIR/${request_name}.body"
  HTTP_STATUS="$(curl --silent --show-error --max-time 30 "$@" \
    --dump-header "$RESPONSE_HEADERS" \
    --output "$RESPONSE_BODY" \
    --write-out '%{http_code}')"
}

assert_status() {
  local expected="$1"
  local actual="$2"
  local description="$3"

  [[ "$actual" == "$expected" ]] || fail "$description returned HTTP $actual; expected $expected."
  pass "$description (HTTP $actual)"
}

assert_header_contains() {
  local headers="$1"
  local expected="$2"

  grep -Fqi -- "$expected" "$headers" || fail "Expected response header '$expected' was missing."
  pass "Response header contains: $expected"
}

assert_body_contains() {
  local body="$1"
  local expected="$2"

  grep -Fq -- "$expected" "$body" || fail "Expected response body to contain '$expected'."
  pass "Response body contains: $expected"
}

signup_account() {
  local scenario="$1"

  TEST_ID="$(date +%s)-$$-${RANDOM}"
  TEST_EMAIL="integration-${scenario}-${TEST_ID}@example.test"
  TEST_PASSWORD='Integration-password-1'
  ORGANIZATION_NAME="Integration ${scenario//-/ } ${TEST_ID}"
  ORGANIZATION_SLUG="${ORGANIZATION_NAME,,}"
  ORGANIZATION_SLUG="${ORGANIZATION_SLUG// /-}"
  COOKIE_JAR="$TEST_TMP_DIR/${TEST_ID}.cookies"

  http_request "${TEST_ID}-signup" \
    --request POST \
    --header "Origin: $BASE_URL" \
    --header 'Content-Type: application/json' \
    --cookie-jar "$COOKIE_JAR" \
    --data "{\"name\":\"$ORGANIZATION_NAME\",\"email\":\"$TEST_EMAIL\",\"password\":\"$TEST_PASSWORD\",\"confirm\":\"$TEST_PASSWORD\"}" \
    "$BASE_URL/api/organizations/create"

  assert_status 201 "$HTTP_STATUS" "Signup"
  assert_header_contains "$RESPONSE_HEADERS" 'content-type: application/json'
  assert_body_contains "$RESPONSE_BODY" '"redirectTo":"/dashboard"'
  assert_header_contains "$RESPONSE_HEADERS" 'set-cookie: session='
  [[ -s "$COOKIE_JAR" ]] || fail "Signup did not store a session cookie."
  pass "Signup stored a session cookie"
}

assert_dashboard_for_cookie_jar() {
  local cookie_jar="$1"

  http_request "${TEST_ID}-dashboard" \
    --cookie "$cookie_jar" \
    "$BASE_URL/dashboard"

  assert_status 200 "$HTTP_STATUS" "Authenticated dashboard"
  assert_body_contains "$RESPONSE_BODY" "$ORGANIZATION_NAME"
}

compose_exec() {
  local project="${COMPOSE_PROJECT_NAME:-}"
  [[ -n "$project" ]] || fail "COMPOSE_PROJECT_NAME must identify the isolated test stack."

  docker compose -p "$project" \
    -f "$ROOT/compose.yaml" \
    -f "$ROOT/compose.integration.yaml" \
    "$@"
}

make_test_png() {
  python3 -c 'import struct,sys,zlib; path,width,height=sys.argv[1],int(sys.argv[2]),int(sys.argv[3]); chunk=lambda name,data: struct.pack(">I",len(data))+name+data+struct.pack(">I",zlib.crc32(name+data)&0xffffffff); raw=b"".join(b"\0"+b"\x55\xaa\xff\xff"*width for _ in range(height)); png=b"\x89PNG\r\n\x1a\n"+chunk(b"IHDR",struct.pack(">IIBBBBB",width,height,8,6,0,0,0))+chunk(b"IDAT",zlib.compress(raw))+chunk(b"IEND",b""); open(path,"wb").write(png)' "$1" "$2" "$3"
}