#!/usr/bin/env bash
# Shared configuration and helpers for ActivityPub protocol test scripts.

# Strict mode
set -euo pipefail

# Paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${SCRIPT_DIR}/output"
mkdir -p "$OUTPUT_DIR"

# Server configuration (override via environment)
: "${PORT:=3000}"
: "${PORT_B:=3001}"
: "${BASE_URL:=http://localhost:${PORT}}"
: "${BASE_URL_B:=http://localhost:${PORT_B}}"

# Content types
AP_CONTENT_TYPE="application/activity+json"
JRD_CONTENT_TYPE="application/jrd+json"

# PIDs of servers we started (for cleanup)
declare -a SERVER_PIDS=()

# Colors for pass/fail output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m'

# ---------------------------------------------------------------------------
# Assertion helpers
# ---------------------------------------------------------------------------

pass() { echo -e "${GREEN}PASS${NC}: $1"; }
fail() { echo -e "${RED}FAIL${NC}: $1"; exit 1; }
info() { echo -e "${YELLOW}INFO${NC}: $1"; }

# Assert HTTP status code
# Usage: assert_status "$STATUS" "200" "Description"
assert_status() {
  local actual="$1" expected="$2" label="$3"
  if [[ "$actual" == "$expected" ]]; then
    pass "$label (HTTP $actual)"
  else
    fail "$label: expected HTTP $expected, got HTTP $actual"
  fi
}

# Assert JSON field equals expected value
# Usage: assert_json "$file" ".field" "expected" "Description"
assert_json() {
  local file="$1" expr="$2" expected="$3" label="$4"
  local actual
  actual=$(jq -r "$expr" < "$file")
  if [[ "$actual" == "$expected" ]]; then
    pass "$label"
  else
    fail "$label: expected '$expected', got '$actual'"
  fi
}

# Assert JSON field contains substring
# Usage: assert_json_contains "$file" ".field" "substring" "Description"
assert_json_contains() {
  local file="$1" expr="$2" substring="$3" label="$4"
  local actual
  actual=$(jq -r "$expr" < "$file")
  if [[ "$actual" == *"$substring"* ]]; then
    pass "$label"
  else
    fail "$label: '$actual' does not contain '$substring'"
  fi
}

# ---------------------------------------------------------------------------
# Server management
# ---------------------------------------------------------------------------

# Start the localhost_server on a given port if not already running
ensure_server_running() {
  local port="${1:-$PORT}"
  local base_url="http://localhost:${port}"

  # Check if already running
  if curl -s --connect-timeout 1 "${base_url}/.well-known/webfinger?resource=acct:alice@localhost:${port}" > /dev/null 2>&1; then
    info "Server already running on port ${port}"
    return 0
  fi

  info "Starting localhost_server on port ${port}..."
  PORT="$port" cargo run -p activitypub-s2s --example localhost_server > "${OUTPUT_DIR}/server-${port}.log" 2>&1 &
  local pid=$!
  SERVER_PIDS+=("$pid")

  # Wait for server to be ready (up to 10 seconds)
  local retries=20
  while ! curl -s --connect-timeout 1 "${base_url}/.well-known/webfinger?resource=acct:alice@localhost:${port}" > /dev/null 2>&1; do
    ((retries--)) || { fail "Server failed to start on port ${port}"; }
    sleep 0.5
  done
  info "Server running on port ${port} (PID: ${pid})"
}

# Stop all servers we started
stop_servers() {
  for pid in "${SERVER_PIDS[@]:-}"; do
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      info "Stopping server (PID: ${pid})..."
      kill "$pid" 2>/dev/null || true
    fi
  done
  SERVER_PIDS=()
}

# ---------------------------------------------------------------------------
# HTTP Signature helpers
# ---------------------------------------------------------------------------

# Compute SHA-256 digest of input
# Usage: echo -n "$body" | compute_digest
compute_digest() {
  echo -n "SHA-256=$(openssl dgst -sha256 -binary | base64)"
}

# Build signing string for HTTP Signature
# Usage: build_signing_string "POST" "/path" "host:value" "date:value" "digest:value"
build_signing_string() {
  local method="$1" path="$2"
  shift 2
  local result="(request-target): $(echo "$method" | tr '[:upper:]' '[:lower:]') ${path}"
  for header in "$@"; do
    result="${result}"$'\n'"${header}"
  done
  echo -n "$result"
}

# Sign a string with RSA-SHA256
# Usage: echo -n "$string" | sign_with_key "$keyfile"
sign_with_key() {
  local keyfile="$1"
  openssl dgst -sha256 -sign "$keyfile" | base64 | tr -d '\n'
}
