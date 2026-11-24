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

pass "Signup, Redis session, protected dashboard, and public profile journey"