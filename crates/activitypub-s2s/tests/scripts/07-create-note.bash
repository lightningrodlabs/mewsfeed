#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.bash"

echo "=== Test: Create Note delivery ==="

# Alice creates a Note and delivers it as a Create activity to Bob's inbox

# Get alice's private key
info "Exporting alice's private key..."
(
  set -x
  curl -s "${BASE_URL}/__debug/key/alice" > "$OUTPUT_DIR/alice-private-key.pem"
)

# Build the Create activity with embedded Note
NOTE_ID="http://localhost:${PORT}/notes/$(date +%s)"
ACTOR="http://localhost:${PORT}/users/alice"

BODY=$(jq -n \
  --arg context "https://www.w3.org/ns/activitystreams" \
  --arg id "http://localhost:${PORT}/activities/create-$(date +%s)" \
  --arg actor "$ACTOR" \
  --arg note_id "$NOTE_ID" \
  --arg content "<p>Hello from MewsFeed! #holochain</p>" \
  '{
    "@context": $context,
    "type": "Create",
    "id": $id,
    "actor": $actor,
    "object": {
      "type": "Note",
      "id": $note_id,
      "attributedTo": $actor,
      "content": $content,
      "to": ["https://www.w3.org/ns/activitystreams#Public"],
      "cc": [($actor + "/followers")]
    },
    "to": ["https://www.w3.org/ns/activitystreams#Public"],
    "cc": [($actor + "/followers")]
  }')

info "Create activity body:"
echo "$BODY" | jq . | tee "$OUTPUT_DIR/create-note-body.json"

# Sign and deliver to Bob's inbox on server B
KEY_ID="http://localhost:${PORT}/users/alice#main-key"
DATE=$(date -u +"%a, %d %b %Y %H:%M:%S GMT")
DIGEST=$(echo -n "$BODY" | compute_digest)
HOST="localhost:${PORT_B}"

SIGNING_STRING=$(build_signing_string "POST" "/users/bob/inbox" \
  "host: ${HOST}" \
  "date: ${DATE}" \
  "digest: ${DIGEST}")

SIGNATURE=$(echo -n "$SIGNING_STRING" | sign_with_key "$OUTPUT_DIR/alice-private-key.pem")

info "Delivering Create activity to Bob..."
STATUS="$(
  set -x
  curl -s -o "$OUTPUT_DIR/create-note-response.json" \
    -w "%{http_code}" \
    -X POST \
    -H "Content-Type: $AP_CONTENT_TYPE" \
    -H "Date: ${DATE}" \
    -H "Host: ${HOST}" \
    -H "Digest: ${DIGEST}" \
    -H "Signature: keyId=\"${KEY_ID}\",algorithm=\"rsa-sha256\",headers=\"(request-target) host date digest\",signature=\"${SIGNATURE}\"" \
    -d "$BODY" \
    "${BASE_URL_B}/users/bob/inbox"
)"
assert_status "$STATUS" "202" "Create Note -> Bob"

# Verify Bob received the Create activity
info "Verifying Bob received the Create..."
(
  set -x
  curl -s "${BASE_URL_B}/__debug/inbox/bob" > "$OUTPUT_DIR/bob-create-received.json"
)
assert_json "$OUTPUT_DIR/bob-create-received.json" '.[-1].type' "Create" "Bob received Create"
assert_json "$OUTPUT_DIR/bob-create-received.json" '.[-1].object.type' "Note" "Create contains Note"

echo "=== Create Note: all tests passed ==="
