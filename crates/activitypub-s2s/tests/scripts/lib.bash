#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.bash"

echo "=== Test: Actor Fetch ==="

STATUS=$(curl -s -o "$OUTPUT_DIR/actor-alice.json" \
  -w "%{http_code}" \
  -H "Accept: application/activity+json" \
  "${BASE_URL}/users/alice")
assert_status "$STATUS" "200" "Actor alice"

jq . < "$OUTPUT_DIR/actor-alice.json"

assert_json "$OUTPUT_DIR/actor-alice.json" ".type" "Person" "Actor type is Person"
assert_json "$OUTPUT_DIR/actor-alice.json" ".preferredUsername" "alice" "Username is alice"
assert_json_contains "$OUTPUT_DIR/actor-alice.json" ".inbox" "/users/alice/inbox" "Inbox URL"
assert_json_contains "$OUTPUT_DIR/actor-alice.json" ".outbox" "/users/alice/outbox" "Outbox URL"
assert_json_contains "$OUTPUT_DIR/actor-alice.json" \
  ".publicKey.publicKeyPem" "BEGIN PUBLIC KEY" "Public key PEM present"

# Unknown user -> 404
STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "Accept: application/activity+json" \
  "${BASE_URL}/users/nobody")
assert_status "$STATUS" "404" "Actor unknown user"

echo "=== Actor Fetch: all tests passed ==="
