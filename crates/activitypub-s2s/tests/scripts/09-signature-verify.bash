#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.bash"

echo "=== Test: Standalone HTTP Signature sign + verify ==="

# Generate a throwaway RSA keypair with openssl
info "Generating test keypair..."
(
  set -x
  openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 \
    -out "$OUTPUT_DIR/test-private.pem" 2>/dev/null
  openssl rsa -in "$OUTPUT_DIR/test-private.pem" -pubout \
    -out "$OUTPUT_DIR/test-public.pem" 2>/dev/null
)

# Construct a signing string
BODY='{"type":"Follow","actor":"http://example/users/test"}'
DATE=$(date -u +"%a, %d %b %Y %H:%M:%S GMT")
DIGEST=$(echo -n "$BODY" | compute_digest)
HOST="example.com"

SIGNING_STRING=$(build_signing_string "POST" "/users/bob/inbox" \
  "host: ${HOST}" \
  "date: ${DATE}" \
  "digest: ${DIGEST}")

info "Signing string:"
echo "$SIGNING_STRING"
echo "---"

# Sign
SIGNATURE=$(echo -n "$SIGNING_STRING" | sign_with_key "$OUTPUT_DIR/test-private.pem")
info "Signature: $SIGNATURE"

# Verify with correct key
info "Verifying with correct key..."
(
  set -x
  echo -n "$SIGNING_STRING" | \
    openssl dgst -sha256 -verify "$OUTPUT_DIR/test-public.pem" \
    -signature <(echo "$SIGNATURE" | base64 -d)
) && pass "Signature verified with correct key" || \
  fail "Signature verification failed with correct key"

# Verify with wrong key (should fail)
info "Generating wrong keypair..."
(
  set -x
  openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 \
    -out "$OUTPUT_DIR/wrong-private.pem" 2>/dev/null
  openssl rsa -in "$OUTPUT_DIR/wrong-private.pem" -pubout \
    -out "$OUTPUT_DIR/wrong-public.pem" 2>/dev/null
)

info "Verifying with wrong key (should fail)..."
(
  set -x
  echo -n "$SIGNING_STRING" | \
    openssl dgst -sha256 -verify "$OUTPUT_DIR/wrong-public.pem" \
    -signature <(echo "$SIGNATURE" | base64 -d)
) 2>/dev/null && fail "Signature should NOT verify with wrong key" || \
  pass "Signature correctly rejected with wrong key"

echo "=== Signature verify: all tests passed ==="
