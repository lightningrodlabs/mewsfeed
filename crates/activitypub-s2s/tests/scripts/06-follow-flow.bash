#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.bash"

echo "=== Test: Full Follow + Accept flow (two servers) ==="

# This script exercises the complete Follow flow:
# 1. Alice (server A) sends Follow to Bob (server B)
# 2. Bob's server verifies and processes
# 3. Bob (server B) sends Accept{Follow} back to Alice (server A)
# 4. Alice's server verifies and processes

# Step 1: Get alice's private key from server A
info "Exporting alice's private key from server A..."
(
  set -x
  curl -s "${BASE_URL}/__debug/key/alice" > "$OUTPUT_DIR/alice-private-key.pem"
)

# Step 2: Alice sends Follow to Bob on server B
FOLLOW_ID="http://localhost:${PORT}/activities/follow-$(date +%s)"
FOLLOW_BODY='{"@context":"https://www.w3.org/ns/activitystreams","type":"Follow","id":"'"${FOLLOW_ID}"'","actor":"http://localhost:'"${PORT}"'/users/alice","object":"http://localhost:'"${PORT_B}"'/users/bob"}'

KEY_ID="http://localhost:${PORT}/users/alice#main-key"
DATE=$(date -u +"%a, %d %b %Y %H:%M:%S GMT")
DIGEST=$(echo -n "$FOLLOW_BODY" | compute_digest)
HOST="localhost:${PORT_B}"

SIGNING_STRING=$(build_signing_string "POST" "/users/bob/inbox" \
  "host: ${HOST}" \
  "date: ${DATE}" \
  "digest: ${DIGEST}")

SIGNATURE=$(echo -n "$SIGNING_STRING" | sign_with_key "$OUTPUT_DIR/alice-private-key.pem")

info "Step 1: Alice -> Follow -> Bob"
STATUS="$(
  set -x
  curl -s -o "$OUTPUT_DIR/follow-to-bob.json" \
    -w "%{http_code}" \
    -X POST \
    -H "Content-Type: $AP_CONTENT_TYPE" \
    -H "Date: ${DATE}" \
    -H "Host: ${HOST}" \
    -H "Digest: ${DIGEST}" \
    -H "Signature: keyId=\"${KEY_ID}\",algorithm=\"rsa-sha256\",headers=\"(request-target) host date digest\",signature=\"${SIGNATURE}\"" \
    -d "$FOLLOW_BODY" \
    "${BASE_URL_B}/users/bob/inbox"
)"
assert_status "$STATUS" "202" "Alice Follow -> Bob"

# Verify Bob received the Follow
(
  set -x
  curl -s "${BASE_URL_B}/__debug/inbox/bob" > "$OUTPUT_DIR/bob-received.json"
)
assert_json "$OUTPUT_DIR/bob-received.json" '.[-1].type' "Follow" "Bob received Follow"

# Step 3: Bob sends Accept back to Alice
info "Exporting bob's private key from server B..."
(
  set -x
  curl -s "${BASE_URL_B}/__debug/key/bob" > "$OUTPUT_DIR/bob-private-key.pem"
)

ACCEPT_ID="http://localhost:${PORT_B}/activities/accept-$(date +%s)"
ACCEPT_BODY='{"@context":"https://www.w3.org/ns/activitystreams","type":"Accept","id":"'"${ACCEPT_ID}"'","actor":"http://localhost:'"${PORT_B}"'/users/bob","object":'"${FOLLOW_BODY}"'}'

KEY_ID_BOB="http://localhost:${PORT_B}/users/bob#main-key"
DATE=$(date -u +"%a, %d %b %Y %H:%M:%S GMT")
DIGEST=$(echo -n "$ACCEPT_BODY" | compute_digest)
HOST="localhost:${PORT}"

SIGNING_STRING=$(build_signing_string "POST" "/users/alice/inbox" \
  "host: ${HOST}" \
  "date: ${DATE}" \
  "digest: ${DIGEST}")

SIGNATURE=$(echo -n "$SIGNING_STRING" | sign_with_key "$OUTPUT_DIR/bob-private-key.pem")

info "Step 2: Bob -> Accept -> Alice"
STATUS="$(
  set -x
  curl -s -o "$OUTPUT_DIR/accept-to-alice.json" \
    -w "%{http_code}" \
    -X POST \
    -H "Content-Type: $AP_CONTENT_TYPE" \
    -H "Date: ${DATE}" \
    -H "Host: ${HOST}" \
    -H "Digest: ${DIGEST}" \
    -H "Signature: keyId=\"${KEY_ID_BOB}\",algorithm=\"rsa-sha256\",headers=\"(request-target) host date digest\",signature=\"${SIGNATURE}\"" \
    -d "$ACCEPT_BODY" \
    "${BASE_URL}/users/alice/inbox"
)"
assert_status "$STATUS" "202" "Bob Accept -> Alice"

# Verify Alice received the Accept
info "Step 3: Verify both received"
(
  set -x
  curl -s "${BASE_URL}/__debug/inbox/alice" > "$OUTPUT_DIR/alice-received.json"
)
assert_json "$OUTPUT_DIR/alice-received.json" '.[-1].type' "Accept" "Alice received Accept"

echo "=== Follow flow: all tests passed ==="
