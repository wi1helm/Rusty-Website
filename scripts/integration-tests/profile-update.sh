#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

signup_account profile-update

PROFILE_DISPLAY_NAME="Updated ${TEST_ID}"
PROFILE_ABOUT="Integration profile details ${TEST_ID}"
PROFILE_LINK_LABEL="Integration Website"
PROFILE_LINK_URL="https://example.test/${TEST_ID}"
PROFILE_PAYLOAD="$(python3 -c 'import json,sys; print(json.dumps({"display_name":sys.argv[1],"about":sys.argv[2],"links":[{"label":sys.argv[3],"url":sys.argv[4]}]}))' \
  "$PROFILE_DISPLAY_NAME" "$PROFILE_ABOUT" "$PROFILE_LINK_LABEL" "$PROFILE_LINK_URL")"

http_request "${TEST_ID}-profile-update" \
  --request POST \
  --header "Origin: $BASE_URL" \
  --header 'Content-Type: application/json' \
  --cookie "$COOKIE_JAR" \
  --data "$PROFILE_PAYLOAD" \
  "$BASE_URL/api/profile/update"
assert_status 303 "$HTTP_STATUS" "Profile update"
assert_header_contains "$RESPONSE_HEADERS" 'location: /profile'

http_request "${TEST_ID}-invalid-profile-update" \
  --request POST \
  --header "Origin: $BASE_URL" \
  --header 'Content-Type: application/json' \
  --cookie "$COOKIE_JAR" \
  --data '{"display_name":"<script>","about":"","links":[]}' \
  "$BASE_URL/api/profile/update"
assert_status 303 "$HTTP_STATUS" "Invalid profile update"
assert_header_contains "$RESPONSE_HEADERS" 'location: /profile?error=invalid-display-name'

http_request "${TEST_ID}-no-change-profile-update" \
  --request POST \
  --header "Origin: $BASE_URL" \
  --header 'Content-Type: application/json' \
  --cookie "$COOKIE_JAR" \
  --data "$PROFILE_PAYLOAD" \
  "$BASE_URL/api/profile/update"
assert_status 303 "$HTTP_STATUS" "Duplicate profile update"
assert_header_contains "$RESPONSE_HEADERS" 'location: /profile'

http_request "${TEST_ID}-updated-public-profile" "$BASE_URL/$ORGANIZATION_SLUG"
assert_status 200 "$HTTP_STATUS" "Updated public profile"
assert_body_contains "$RESPONSE_BODY" "$PROFILE_DISPLAY_NAME"
assert_body_contains "$RESPONSE_BODY" "$PROFILE_ABOUT"
assert_body_contains "$RESPONSE_BODY" "$PROFILE_LINK_LABEL"
assert_body_contains "$RESPONSE_BODY" "$PROFILE_LINK_URL"

pass "Profile update, invalid input, no-change behavior, and public persistence checks"