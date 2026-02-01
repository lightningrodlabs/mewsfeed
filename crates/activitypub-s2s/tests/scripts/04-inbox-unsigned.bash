#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.bash"

echo "=== Test: Inbox POST without signature ==="

BODY='{"@context":"https://www.w3.org/ns/activitystreams","type":"Follow","id":"http://remote/follow/1","actor":"http://remote/users/eve","object":"http://localhost:'"${PORT}"'/users/alice"}'

# POST without Signature header should be rejected
STATUS="$(
  set -x
  curl -s -o "$OUTPUT_DIR/inbox-unsigned.json" \
    -w "%{http_code}" \
    -X POST \
    -H "Content-Type: $AP_CONTENT_TYPE" \
    -d "$BODY" \
    "${BASE_URL}/users/alice/inbox"
)"
assert_status "$STATUS" "401" "Inbox unsigned -> 401"

echo "=== Inbox unsigned: all tests passed ==="
