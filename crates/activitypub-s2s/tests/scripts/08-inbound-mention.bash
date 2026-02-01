#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.bash"

echo "=== Test: Inbound Mention ==="

# Bob (server B) creates a Note mentioning Alice and delivers it to Alice's inbox

# Get bob's private key from server B
info "Exporting bob's private key..."
(
  set -x
  curl -s "${BASE_URL_B}/__debug/key/bob" > "$OUTPUT_DIR/bob-private-key.pem"
)

# Build the Create activity with a Note that mentions Alice
NOTE_ID="http://localhost:${PORT_B}/notes/mention-$(date +%s)"
BOB_ACTOR="http://localhost:${PORT_B}/users/bob"
ALICE_ACTOR="http://localhost:${PORT}/users/alice"

BODY=$(jq -n \
  --arg context "https://www.w3.org/ns/activitystreams" \
  --arg id "http://localhost:${PORT_B}/activities/create-mention-$(date +%s)" \
  --arg actor "$BOB_ACTOR" \
  --arg note_id "$NOTE_ID" \
  --arg alice_actor "$ALICE_ACTOR" \
  --arg content "<p>Hey <a href=\"${ALICE_ACTOR}\">@alice</a>, check this out!</p>" \
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
      "to": [$alice_actor],
      "cc": ["https://www.w3.org/ns/activitystreams#Public"],
      "tag": [
        {
          "type": "Mention",
          "href": $alice_actor,
          "name": "@alice@localhost:'"${PORT}"'"
        }
      ]
    },
    "to": [$alice_actor],
    "cc": ["https://www.w3.org/ns/activitystreams#Public"]
  }')

info "Mention activity body:"
echo "$BODY" | jq . | tee "$OUTPUT_DIR/mention-body.json"

# Sign and deliver to Alice's inbox on server A
KEY_ID="http://localhost:${PORT_B}/users/bob#main-key"
DATE=$(date -u +"%a, %d %b %Y %H:%M:%S GMT")
DIGEST=$(echo -n "$BODY" | compute_digest)
HOST="localhost:${PORT}"

SIGNING_STRING=$(build_signing_string "POST" "/users/alice/inbox" \
  "host: ${HOST}" \
  "date: ${DATE}" \
  "digest: ${DIGEST}")

SIGNATURE=$(echo -n "$SIGNING_STRING" | sign_with_key "$OUTPUT_DIR/bob-private-key.pem")

info "Delivering Mention to Alice..."
STATUS="$(
  set -x
  curl -s -o "$OUTPUT_DIR/mention-response.json" \
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
assert_status "$STATUS" "202" "Mention -> Alice"

# Verify Alice received the mention
info "Verifying Alice received the mention..."
(
  set -x
  curl -s "${BASE_URL}/__debug/inbox/alice" > "$OUTPUT_DIR/alice-mention-received.json"
)
assert_json "$OUTPUT_DIR/alice-mention-received.json" '.[-1].type' "Create" "Alice received Create"
assert_json "$OUTPUT_DIR/alice-mention-received.json" '.[-1].object.tag[0].type' "Mention" "Note contains Mention tag"

echo "=== Inbound Mention: all tests passed ==="
