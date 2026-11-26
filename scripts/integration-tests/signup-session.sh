#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

signup_account signup-session
assert_dashboard_for_cookie_jar "$COOKIE_JAR"

http_request "${TEST_ID}-public-profile" "$BASE_URL/$ORGANIZATION_SLUG"
assert_status 200 "$HTTP_STATUS" "Public organization profile"
assert_body_contains "$RESPONSE_BODY" "$ORGANIZATION_NAME"
assert_body_contains "$RESPONSE_BODY" 'id="profile-display-name"'

http_request "${TEST_ID}-duplicate-email" \
  --request POST \
  --header "Origin: $BASE_URL" \
  --header 'Content-Type: application/json' \
  --data "{\"name\":\"Duplicate Email ${TEST_ID}\",\"email\":\"$TEST_EMAIL\",\"password\":\"$TEST_PASSWORD\",\"confirm\":\"$TEST_PASSWORD\"}" \
  "$BASE_URL/api/organizations/create"
assert_status 409 "$HTTP_STATUS" "Duplicate-email signup response"
assert_body_contains "$RESPONSE_BODY" '"code":"signup-failed"'

http_request "${TEST_ID}-reserved-slug" \
  --request POST \
  --header "Origin: $BASE_URL" \
  --header 'Content-Type: application/json' \
  --data "{\"name\":\"Login\",\"email\":\"reserved-${TEST_ID}@example.test\",\"password\":\"$TEST_PASSWORD\",\"confirm\":\"$TEST_PASSWORD\"}" \
  "$BASE_URL/api/organizations/create"
assert_status 400 "$HTTP_STATUS" "Reserved-slug signup response"
assert_body_contains "$RESPONSE_BODY" '"code":"invalid-slug"'

pass "Signup, duplicate-email handling, reserved slugs, sessions, and public profiles"