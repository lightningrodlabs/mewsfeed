#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.bash"

echo "=== Test: WebFinger Discovery ==="

# Valid request for alice
STATUS=$(curl -s -o "$OUTPUT_DIR/webfinger-alice.json" \
  -w "%{http_code}" \
  "${BASE_URL}/.well-known/webfinger?resource=acct:alice@localhost:${PORT}")
assert_status "$STATUS" "200" "WebFinger alice"

jq . < "$OUTPUT_DIR/webfinger-alice.json"

assert_json "$OUTPUT_DIR/webfinger-alice.json" \
  ".subject" "acct:alice@localhost:${PORT}" \
  "WebFinger subject matches"

assert_json "$OUTPUT_DIR/webfinger-alice.json" \
  '.links[] | select(.rel=="self") | .type' "application/activity+json" \
  "WebFinger self link type"

assert_json_contains "$OUTPUT_DIR/webfinger-alice.json" \
  '.links[] | select(.rel=="self") | .href' "/users/alice" \
  "WebFinger self link href contains /users/alice"

# Unknown user -> 404
STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
  "${BASE_URL}/.well-known/webfinger?resource=acct:nobody@localhost:${PORT}")
assert_status "$STATUS" "404" "WebFinger unknown user"

# Missing resource param -> 400
STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
  "${BASE_URL}/.well-known/webfinger")
assert_status "$STATUS" "400" "WebFinger missing resource"

echo "=== WebFinger: all tests passed ==="
