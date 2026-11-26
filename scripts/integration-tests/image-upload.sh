#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

signup_account image-upload
IMAGE_FILE="$TEST_TMP_DIR/${TEST_ID}.png"
make_test_png "$IMAGE_FILE" 200 200

http_request "${TEST_ID}-image-upload" \
  --request POST \
  --header "Origin: $BASE_URL" \
  --cookie "$COOKIE_JAR" \
  --form 'image_type=logo' \
  --form "image=@$IMAGE_FILE;type=image/png" \
  "$BASE_URL/api/profile/image"
assert_status 204 "$HTTP_STATUS" "Valid profile image upload"

IMAGE_RECORD="$(compose_exec exec -T postgres psql \
  -U "$POSTGRES_USER" \
  -d "$POSTGRES_DB" \
  -Atqc "SELECT storage_key || '|' || content_type FROM organization_profile_images WHERE organization_id = (SELECT id FROM organizations WHERE slug = '$ORGANIZATION_SLUG') AND image_type = 'logo' ORDER BY id DESC LIMIT 1")"
[[ -n "$IMAGE_RECORD" ]] || fail "No logo image metadata was saved to PostgreSQL."
IMAGE_KEY="${IMAGE_RECORD%%|*}"
IMAGE_CONTENT_TYPE="${IMAGE_RECORD#*|}"
[[ "$IMAGE_CONTENT_TYPE" == image/webp ]] || fail "Expected normalized image/webp metadata; found $IMAGE_CONTENT_TYPE."
pass "Uploaded image metadata was saved as image/webp"

IMAGE_URL="/images/${IMAGE_KEY}"
IMAGE_REQUEST_URL="${BASE_URL}${IMAGE_URL}"
http_request "${TEST_ID}-public-profile-image" "$BASE_URL/$ORGANIZATION_SLUG"
assert_status 200 "$HTTP_STATUS" "Public profile after image upload"
assert_body_contains "$RESPONSE_BODY" "$IMAGE_URL"

RETRIEVED_IMAGE="$TEST_TMP_DIR/${TEST_ID}-retrieved.webp"
IMAGE_RESPONSE="$(curl --silent --show-error --max-time 30 \
  --output "$RETRIEVED_IMAGE" \
  --write-out '%{http_code}|%{content_type}' \
  "$IMAGE_REQUEST_URL")"
IFS='|' read -r IMAGE_STATUS IMAGE_RESPONSE_TYPE <<< "$IMAGE_RESPONSE"
if [[ "$IMAGE_STATUS" != 200 ]]; then
  echo "Image proxy request $IMAGE_REQUEST_URL for object key $IMAGE_KEY returned HTTP $IMAGE_STATUS ($IMAGE_RESPONSE_TYPE)." >&2
  head -c 500 "$RETRIEVED_IMAGE" >&2 || true
  echo >&2
  fail "Expected uploaded image to be retrievable through the app proxy."
fi
[[ "$IMAGE_RESPONSE_TYPE" == image/webp ]] || fail "Expected uploaded object to return image/webp; found $IMAGE_RESPONSE_TYPE."
[[ -s "$RETRIEVED_IMAGE" ]] || fail "Uploaded image object was empty when fetched from S3-compatible storage."
pass "Uploaded image was retrieved from object storage"

http_request "${TEST_ID}-malformed-multipart" \
  --request POST \
  --header "Origin: $BASE_URL" \
  --header 'Content-Type: multipart/form-data; boundary=broken-boundary' \
  --cookie "$COOKIE_JAR" \
  --data-binary 'this is not a multipart body' \
  "$BASE_URL/api/profile/image"
assert_status 400 "$HTTP_STATUS" "Malformed multipart upload"
assert_body_contains "$RESPONSE_BODY" '"error"'

UNSUPPORTED_FILE="$TEST_TMP_DIR/${TEST_ID}.txt"
printf 'not an image' > "$UNSUPPORTED_FILE"
http_request "${TEST_ID}-unsupported-image" \
  --request POST \
  --header "Origin: $BASE_URL" \
  --cookie "$COOKIE_JAR" \
  --form 'image_type=logo' \
  --form "image=@$UNSUPPORTED_FILE;type=text/plain" \
  "$BASE_URL/api/profile/image"
assert_status 400 "$HTTP_STATUS" "Unsupported image content type"

OVERSIZED_FILE="$TEST_TMP_DIR/${TEST_ID}-oversized.png"
truncate -s 10485761 "$OVERSIZED_FILE"
http_request "${TEST_ID}-oversized-image" \
  --request POST \
  --header "Origin: $BASE_URL" \
  --cookie "$COOKIE_JAR" \
  --form 'image_type=logo' \
  --form "image=@$OVERSIZED_FILE;type=image/png" \
  "$BASE_URL/api/profile/image"
assert_status 413 "$HTTP_STATUS" "Oversized image upload"

pass "Image upload, database metadata, S3 retrieval, and invalid upload checks"