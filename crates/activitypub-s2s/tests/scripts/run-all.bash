#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "${SCRIPT_DIR}/lib.bash"

trap stop_servers EXIT

ensure_server_running "$PORT"

echo "========================================="
echo " ActivityPub Localhost Protocol Tests"
echo " Base URL: ${BASE_URL}"
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

# Single-server tests
run_test "${SCRIPT_DIR}/01-webfinger.bash"
run_test "${SCRIPT_DIR}/02-actor-fetch.bash"
run_test "${SCRIPT_DIR}/03-outbox.bash"
run_test "${SCRIPT_DIR}/04-inbox-unsigned.bash"
run_test "${SCRIPT_DIR}/09-signature-verify.bash"

# Two-server tests (require PORT_B to be set)
if [[ -n "${PORT_B:-}" ]]; then
  ensure_server_running "$PORT_B"
  run_test "${SCRIPT_DIR}/05-inbox-signed.bash"
else
  echo ""
  echo "(Skipping two-server tests: set PORT_B to enable)"
fi

echo ""
echo "========================================="
echo " Results: ${PASS} passed, ${FAIL} failed"
echo "========================================="

[[ "$FAIL" -eq 0 ]] || exit 1
