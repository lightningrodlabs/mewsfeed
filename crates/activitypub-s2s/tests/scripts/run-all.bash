#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "${SCRIPT_DIR}/lib.bash"

trap stop_servers EXIT

# Start servers
ensure_server_running "$PORT"
ensure_server_running "$PORT_B"

echo "========================================="
echo " ActivityPub Localhost Protocol Tests"
echo " Server A: ${BASE_URL}"
echo " Server B: ${BASE_URL_B}"
echo "========================================="
echo ""

PASS=0
FAIL=0

run_test() {
  local script="$1"
  echo ""
  echo "--- Running: $(basename "$script") ---"
  if bash "$script"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "^^^ FAILED: $(basename "$script") ^^^"
  fi
}

# All tests in order
TESTS=(
  # Single-server tests
  "01-webfinger.bash"
  "02-actor-fetch.bash"
  "03-outbox.bash"
  "04-inbox-unsigned.bash"
  "09-signature-verify.bash"
  # Two-server tests
  "05-inbox-signed.bash"
  "06-follow-flow.bash"
  "07-create-note.bash"
  "08-inbound-mention.bash"
)

for test in "${TESTS[@]}"; do
  run_test "${SCRIPT_DIR}/${test}"
done

echo ""
echo "========================================="
echo " Results: ${PASS} passed, ${FAIL} failed"
echo "========================================="

[[ "$FAIL" -eq 0 ]] || exit 1
