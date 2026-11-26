#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

signup_account login-access

WRONG_PASSWORD='Incorrect-password-1'
http_request "${TEST_ID}-wrong-login" \
  --request POST \
  --header "Origin: $BASE_URL" \
  --header 'Content-Type: application/json' \
  --data "{\"email\":\"$TEST_EMAIL\",\"password\":\"$WRONG_PASSWORD\"}" \
  "$BASE_URL/api/auth/login"
assert_status 401 "$HTTP_STATUS" "Login with an incorrect password"
assert_header_contains "$RESPONSE_HEADERS" 'content-type: application/json'
assert_body_contains "$RESPONSE_BODY" '"code":"invalid-credentials"'
if grep -Fqi 'set-cookie: session=' "$RESPONSE_HEADERS"; then
  fail "Login with an incorrect password issued a session cookie."
fi
pass "Rejected login did not issue a session cookie"

http_request "${TEST_ID}-login-page" "$BASE_URL/login"
assert_status 200 "$HTTP_STATUS" "Server-rendered login page"
assert_header_contains "$RESPONSE_HEADERS" 'content-type: text/html'
assert_body_contains "$RESPONSE_BODY" 'action="/api/auth/login"'

http_request "${TEST_ID}-browser-not-found" \
  --header 'Accept: text/html' \
  "$BASE_URL/not-a-real-page-${TEST_ID}"
assert_status 404 "$HTTP_STATUS" "Browser navigation to a missing page"
assert_header_contains "$RESPONSE_HEADERS" 'content-type: text/html'
assert_body_contains "$RESPONSE_BODY" 'Page not found'

LOGIN_COOKIE_JAR="$TEST_TMP_DIR/${TEST_ID}-login.cookies"
http_request "${TEST_ID}-valid-login" \
  --request POST \
  --header "Origin: $BASE_URL" \
  --header 'Content-Type: application/json' \
  --cookie-jar "$LOGIN_COOKIE_JAR" \
  --data "{\"email\":\"$TEST_EMAIL\",\"password\":\"$TEST_PASSWORD\"}" \
  "$BASE_URL/api/auth/login"
assert_status 200 "$HTTP_STATUS" "Login with valid credentials"
assert_body_contains "$RESPONSE_BODY" '"redirectTo":"/dashboard"'
assert_header_contains "$RESPONSE_HEADERS" 'set-cookie: session='
assert_dashboard_for_cookie_jar "$LOGIN_COOKIE_JAR"

assert_login_redirect() {
  local session_state="$1"
  local route="$2"
  local method="$3"
  local session_id="$4"
  local name="${TEST_ID}-${session_state}-${method}-${route//\//-}"
  local curl_args=(--request "$method")

  if [[ -n "$session_id" ]]; then
    curl_args+=(--header "Cookie: session=$session_id")
  fi

  if [[ "$method" == POST ]]; then
    curl_args+=(
      --header "Origin: $BASE_URL"
      --header 'Content-Type: application/json'
      --data '{"display_name":"Unauthenticated","about":"","links":[]}'
    )
  fi

  http_request "$name" "${curl_args[@]}" "$BASE_URL$route"
  assert_status 303 "$HTTP_STATUS" "$session_state request to $route"
  assert_header_contains "$RESPONSE_HEADERS" 'location: /login'
}

assert_api_auth_error() {
  local session_state="$1"
  local route="$2"
  local method="$3"
  local session_id="$4"
  local name="${TEST_ID}-${session_state}-${method}-${route//\//-}"
  local curl_args=(--request "$method")

  if [[ -n "$session_id" ]]; then
    curl_args+=(--header "Cookie: session=$session_id")
  fi

  curl_args+=(
    --header "Origin: $BASE_URL"
    --header 'Content-Type: application/json'
    --data '{"display_name":"Unauthenticated","about":"","links":[]}'
  )

  http_request "$name" "${curl_args[@]}" "$BASE_URL$route"
  assert_status 401 "$HTTP_STATUS" "$session_state request to $route"
  assert_header_contains "$RESPONSE_HEADERS" 'content-type: application/json'
  assert_body_contains "$RESPONSE_BODY" '"code":"authentication-required"'
}

EXPIRED_SESSION_ID="expired-${TEST_ID}"
compose_exec exec -T redis redis-cli SET \
  "session:$EXPIRED_SESSION_ID" \
  "{\"id\":\"$EXPIRED_SESSION_ID\",\"member_id\":1}" \
  EX 1 >/dev/null
sleep 2

for route in /dashboard /profile; do
  assert_login_redirect missing "$route" GET ''
  assert_login_redirect invalid "$route" GET 'not-a-valid-session'
  assert_login_redirect expired "$route" GET "$EXPIRED_SESSION_ID"
done

for route in /api/profile/update /api/profile/update-graph /api/profile/image; do
  assert_api_auth_error missing "$route" POST ''
  assert_api_auth_error invalid "$route" POST 'not-a-valid-session'
  assert_api_auth_error expired "$route" POST "$EXPIRED_SESSION_ID"
done

pass "Valid login, rejected credentials, and protected-route session checks"