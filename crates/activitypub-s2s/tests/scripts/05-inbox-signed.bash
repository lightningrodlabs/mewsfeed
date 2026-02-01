#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.bash"

echo "=== Test: Inbox POST with valid signature ==="

# This test uses a second server on PORT_B.
# Bob (on server B) sends a signed Follow to alice (on server A).

# Step 1: Fetch bob's actor from server B to verify it's up
info "Fetching bob's actor from server B..."
(
  set -x
  curl -s -H "Accept: $AP_CONTENT_TYPE" \
    "${BASE_URL_B}/users/bob" > "$OUTPUT_DIR/actor-bob.json"
)
assert_json "$OUTPUT_DIR/actor-bob.json" ".preferredUsername" "bob" "Bob actor exists on server B"

# Step 2: Export bob's private key from server B (debug endpoint)
info "Exporting bob's private key..."
(
  set -x
  curl -s "${BASE_URL_B}/__debug/key/bob" > "$OUTPUT_DIR/bob-private-key.pem"
)

# Step 3: Build and sign the Follow activity
BODY='{"@context":"https://www.w3.org/ns/activitystreams","type":"Follow","id":"http://localhost:'"${PORT_B}"'/activities/follow-1","actor":"http://localhost:'"${PORT_B}"'/users/bob","object":"http://localhost:'"${PORT}"'/users/alice"}'

KEY_ID="http://localhost:${PORT_B}/users/bob#main-key"
DATE=$(date -u +"%a, %d %b %Y %H:%M:%S GMT")
DIGEST=$(echo -n "$BODY" | compute_digest)
HOST="localhost:${PORT}"
REQUEST_TARGET="post /users/alice/inbox"

SIGNING_STRING=$(build_signing_string "POST" "/users/alice/inbox" \
  "host: ${HOST}" \
  "date: ${DATE}" \
  "digest: ${DIGEST}")

info "Signing string:"
echo "$SIGNING_STRING"
echo "---"

SIGNATURE=$(echo -n "$SIGNING_STRING" | sign_with_key "$OUTPUT_DIR/bob-private-key.pem")

# Step 4: Send signed POST to inbox
STATUS="$(
  set -x
  curl -s -o "$OUTPUT_DIR/inbox-signed.json" \
    -w "%{http_code}" \
    -X POST \
    -H "Content-Type: $AP_CONTENT_TYPE" \
    -H "Date: ${DATE}" \
    -H "Host: ${HOST}" \
    -H "Digest: ${DIGEST}" \
    -H "Signature: keyId=\"${KEY_ID}\",algorithm=\"rsa-sha256\",headers=\"(request-target) host date digest\",signature=\"${SIGNATURE}\"" \
    -d "$BODY" \
    "${BASE_URL}/users/alice/inbox"
)"
assert_status "$STATUS" "202" "Inbox signed Follow -> 202"

# Step 5: Verify the activity was received via debug endpoint
info "Checking received activities..."
(
  set -x
  curl -s "${BASE_URL}/__debug/inbox/alice" > "$OUTPUT_DIR/inbox-alice-received.json"
)
assert_json "$OUTPUT_DIR/inbox-alice-received.json" '.[0].type' "Follow" \
  "Received activity is a Follow"

echo "=== Inbox signed: all tests passed ==="
