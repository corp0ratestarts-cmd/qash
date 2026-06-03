#!/usr/bin/env bash
# Pure QASH Absence Guards
# Fails if any forbidden regulated/disclosure/fee-market concept appears in the repo.
# Run on every PR: CI blocks merge if this exits non-zero.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FAIL=0
RESULTS=()

check_absent() {
    local term="$1"
    local scope="${2:-.}"
    local exclude="${3:-}"

    local grep_args=(-r --include="*.rs" --include="*.toml" --include="*.md" -l)
    if [[ -n "$exclude" ]]; then
        grep_args+=(--exclude-dir="$exclude")
    fi

    if grep -q "${grep_args[@]}" -- "$term" "$REPO_ROOT/$scope" 2>/dev/null; then
        echo "FAIL: '$term' found in $scope"
        grep "${grep_args[@]}" -- "$term" "$REPO_ROOT/$scope" || true
        RESULTS+=("FAIL: $term")
        FAIL=1
    else
        RESULTS+=("PASS: $term absent")
    fi
}

echo "=== Pure QASH Absence Guards ==="
echo "Repo: $REPO_ROOT"
echo ""

# ── Regulated / disclosure profile concepts ──────────────────────────────────
check_absent "ClassIV"
check_absent "class_iv"
check_absent "class-iv"
check_absent "lawful_basis"
check_absent "LawfulBasis"
check_absent "lawful-basis"
check_absent "lawful basis"
check_absent "regulated_disclosure"
check_absent "RegulatedDisclosure"
check_absent "disclosure_key"
check_absent "DisclosureKey"
check_absent "viewing_key"     # receipt viewing key for regulated disclosure
check_absent "disclosure_domain"
check_absent "receipt_disclosure"
check_absent "RegulatoryAuthority"

# ── Fee market / MEV concepts ─────────────────────────────────────────────────
check_absent "priority_fee"
check_absent "PriorityFee"
check_absent "base_fee_plus_tip"
check_absent "gas_price"
check_absent "gas_tip"
check_absent "mempool"
check_absent "builder"         # block builder MEV pattern
check_absent "sequencer"       # centralized sequencer MEV pattern

# MEV is allowed ONLY in docs/claims/ and docs/spec/08_tokenomics.md (explains absence)
if grep -r --include="*.rs" --include="*.toml" -l "MEV" "$REPO_ROOT" 2>/dev/null | \
   grep -v "docs/claims/" | grep -v "docs/spec/08_tokenomics" | grep -q .; then
    echo "FAIL: 'MEV' found outside allowed docs paths"
    grep -r --include="*.rs" --include="*.toml" -l "MEV" "$REPO_ROOT" | \
        grep -v "docs/claims/" | grep -v "docs/spec/08_tokenomics" || true
    RESULTS+=("FAIL: MEV in disallowed path")
    FAIL=1
else
    RESULTS+=("PASS: MEV only in allowed docs paths")
fi

# ── Raw graph persistence concepts ────────────────────────────────────────────
check_absent "raw_tx_wal"
check_absent "RawTxWal"
check_absent "receipt_plaintext"
check_absent "peer_ip"
check_absent "PeerIp"
check_absent "socket_addr" "crates"   # forbidden in Rust crates (PAL/consensus)

# ── EphemeralEnvelope serialization (must not be implemented) ────────────────
check_absent "impl Serialize for EphemeralEnvelope"
check_absent "impl Debug for EphemeralEnvelope"
check_absent "impl Display for EphemeralEnvelope"
check_absent "impl Clone for EphemeralEnvelope"

# ── Summary ───────────────────────────────────────────────────────────────────
echo ""
echo "=== Results ==="
for r in "${RESULTS[@]}"; do
    echo "  $r"
done
echo ""

if [[ $FAIL -eq 0 ]]; then
    echo "Pure QASH Absence Guards: ALL PASS"
    exit 0
else
    echo "Pure QASH Absence Guards: FAILED"
    exit 1
fi
