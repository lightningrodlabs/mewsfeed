#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.bash"

echo "=== Test: Outbox ==="

STATUS=$(curl -s -o "$OUTPUT_DIR/outbox-alice.json" \
  -w "%{http_code}" \
  -H "Accept: application/activity+json" \
  "${BASE_URL}/users/alice/outbox")
assert_status "$STATUS" "200" "Outbox alice"

jq . < "$OUTPUT_DIR/outbox-alice.json"

assert_json "$OUTPUT_DIR/outbox-alice.json" ".type" "OrderedCollection" "Outbox type"
assert_json_contains "$OUTPUT_DIR/outbox-alice.json" ".id" "/users/alice/outbox" "Outbox id"

echo "=== Outbox: all tests passed ==="
