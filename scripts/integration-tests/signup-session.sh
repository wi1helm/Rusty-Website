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
  --data-urlencode "name=Duplicate Email ${TEST_ID}" \
  --data-urlencode "email=$TEST_EMAIL" \
  --data-urlencode "password=$TEST_PASSWORD" \
  --data-urlencode "confirm=$TEST_PASSWORD" \
  "$BASE_URL/api/organizations/create"
assert_status 303 "$HTTP_STATUS" "Duplicate-email signup response"
assert_header_contains "$RESPONSE_HEADERS" 'location: /create?error=signup-failed'

http_request "${TEST_ID}-reserved-slug" \
  --request POST \
  --header "Origin: $BASE_URL" \
  --data-urlencode "name=Login" \
  --data-urlencode "email=reserved-${TEST_ID}@example.test" \
  --data-urlencode "password=$TEST_PASSWORD" \
  --data-urlencode "confirm=$TEST_PASSWORD" \
  "$BASE_URL/api/organizations/create"
assert_status 303 "$HTTP_STATUS" "Reserved-slug signup response"
assert_header_contains "$RESPONSE_HEADERS" 'location: /create?error=invalid-slug'

pass "Signup, duplicate-email handling, reserved slugs, sessions, and public profiles"