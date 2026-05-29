#!/usr/bin/env bash
# verify_sigstore_attestation.sh — verify a QASH binary against the Sigstore
# Rekor transparency log.
#
# Usage:
#   bash scripts/verify_sigstore_attestation.sh <binary> <commit-sha>
#
# Prerequisites (installed in CI, optional locally):
#   cosign  >= 2.0   (https://github.com/sigstore/cosign)
#   rekor-cli        (https://github.com/sigstore/rekor)
#
# The script:
#   1. Computes SHA-256 of <binary>.
#   2. Verifies the Rekor bundle for <commit-sha> against the binary hash.
#   3. Checks the OIDC issuer and identity match the QASH release workflow.
#
# If cosign is not installed, the script prints the hash and a manual
# verification note and exits 0 (non-blocking for local developer use).
# In CI (COSIGN_REQUIRED=1) it exits 1 if cosign is missing.
#
# Exit codes:
#   0 — verified (or cosign absent in non-required mode)
#   1 — verification failed or prerequisites missing in required mode

set -euo pipefail

BINARY="${1:-}"
COMMIT="${2:-}"

if [[ -z "$BINARY" || -z "$COMMIT" ]]; then
    echo "Usage: $0 <binary> <commit-sha>" >&2
    exit 1
fi

if [[ ! -f "$BINARY" ]]; then
    echo "ERROR: binary not found: $BINARY" >&2
    exit 1
fi

HASH=$(sha256sum "$BINARY" | awk '{print $1}')
echo "Binary SHA-256: $HASH"
echo "Commit:         $COMMIT"

# ── Check for cosign ──────────────────────────────────────────────────────────

if ! command -v cosign &>/dev/null; then
    if [[ "${COSIGN_REQUIRED:-0}" == "1" ]]; then
        echo "ERROR: cosign is required (COSIGN_REQUIRED=1) but not installed." >&2
        exit 1
    fi
    echo "NOTE: cosign not installed — skipping Sigstore verification."
    echo "      To verify manually:"
    echo "        cosign verify-blob \\"
    echo "          --certificate-oidc-issuer https://token.actions.githubusercontent.com \\"
    echo "          --certificate-identity \"https://github.com/corp0ratestarts-cmd/qash/.github/workflows/release-attestation.yml@refs/heads/main\" \\"
    echo "          --bundle \"rekor-bundle-${COMMIT}.json\" \\"
    echo "          \"$BINARY\""
    exit 0
fi

# ── Locate Rekor bundle ───────────────────────────────────────────────────────

BUNDLE="rekor-bundle-${COMMIT}.json"
if [[ ! -f "$BUNDLE" ]]; then
    # Try artifacts/attestations/ if bundle not in cwd
    if [[ -f "artifacts/attestations/rekor-bundle-${COMMIT}.json" ]]; then
        BUNDLE="artifacts/attestations/rekor-bundle-${COMMIT}.json"
    else
        echo "ERROR: Rekor bundle not found: ${BUNDLE}" >&2
        echo "       Download from the release artifacts or Rekor log." >&2
        exit 1
    fi
fi

# ── Run cosign verify-blob ────────────────────────────────────────────────────

cosign verify-blob \
    --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
    --certificate-identity "https://github.com/corp0ratestarts-cmd/qash/.github/workflows/release-attestation.yml@refs/heads/main" \
    --bundle "$BUNDLE" \
    "$BINARY"

echo ""
echo "PASS: Binary $HASH verified against commit $COMMIT in Sigstore Rekor."
