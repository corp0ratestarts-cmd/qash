#!/usr/bin/env bash
# Pure QASH Absence Guards
# Fails if any forbidden regulated/disclosure/fee-market concept appears in Rust
# source code (crates/) or TOML configuration files.
#
# SCOPE RATIONALE:
#   - .rs files: scanned only under crates/ (the implementation).
#     Documentation (docs/, CLAUDE.md) and tooling (xtask/) legitimately name
#     these concepts to explain their absence — scanning them causes false positives.
#   - .toml files: scanned repo-wide (configuration must not re-enable any concept).
#     GENESIS_CONSTANTS.toml uses `*_enabled = false` flags — these are legitimate
#     and tested by `cargo xtask check-tokenomics` separately.
#
# Run on every PR: CI blocks merge if this exits non-zero.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FAIL=0
RESULTS=()

# check_absent_rs: checks only *.rs files under crates/ (implementation scope).
check_absent_rs() {
    local term="$1"
    if grep -r --include="*.rs" -ql -- "$term" "$REPO_ROOT/crates" 2>/dev/null; then
        echo "FAIL: '$term' found in crates/"
        grep -r --include="*.rs" -l -- "$term" "$REPO_ROOT/crates" || true
        RESULTS+=("FAIL: $term")
        FAIL=1
    else
        RESULTS+=("PASS: $term absent from crates/")
    fi
}

# check_absent_toml: checks *.toml files repo-wide (excluding GENESIS_CONSTANTS boolean flags).
check_absent_toml() {
    local term="$1"
    # Exclude lines that are `term = false` (disabled flags are fine)
    if grep -r --include="*.toml" -- "$term" "$REPO_ROOT" 2>/dev/null | grep -v "= false" | grep -q .; then
        echo "FAIL: '$term' appears enabled in a .toml file"
        grep -r --include="*.toml" -- "$term" "$REPO_ROOT" | grep -v "= false" || true
        RESULTS+=("FAIL: $term in toml")
        FAIL=1
    else
        RESULTS+=("PASS: $term not enabled in toml")
    fi
}

echo "=== Pure QASH Absence Guards ==="
echo "Repo: $REPO_ROOT"
echo ""

# ── Regulated / disclosure profile concepts (check Rust implementation only) ───
check_absent_rs "ClassIV"
check_absent_rs "class_iv"
check_absent_rs "lawful_basis"
check_absent_rs "LawfulBasis"
check_absent_rs "regulated_disclosure"
check_absent_rs "RegulatedDisclosure"
check_absent_rs "disclosure_key"
check_absent_rs "DisclosureKey"
check_absent_rs "viewing_key"
check_absent_rs "disclosure_domain"
check_absent_rs "receipt_disclosure"
check_absent_rs "RegulatoryAuthority"

# ── Fee market / MEV concepts (check Rust implementation only) ───────────────
check_absent_rs "priority_fee"
check_absent_rs "PriorityFee"
check_absent_rs "base_fee_plus_tip"
check_absent_rs "gas_price"
check_absent_rs "gas_tip"
check_absent_rs "mempool"

# MEV is allowed ONLY in docs/claims/ and docs/spec/08_tokenomics.md
if grep -r --include="*.rs" -ql "MEV" "$REPO_ROOT/crates" 2>/dev/null; then
    echo "FAIL: 'MEV' found in crates/ Rust source"
    grep -r --include="*.rs" -l "MEV" "$REPO_ROOT/crates" || true
    RESULTS+=("FAIL: MEV in crates/ Rust source")
    FAIL=1
else
    RESULTS+=("PASS: MEV absent from crates/ Rust source")
fi

# ── Raw graph persistence (check Rust implementation only) ────────────────────
check_absent_rs "struct RawTxWal"
check_absent_rs "struct PeerIpWal"
check_absent_rs "receipt_plaintext"
check_absent_rs "Vec<RawTx"

# socket_addr: allowed in comments only; fail if it appears as a field/type
if grep -r --include="*.rs" -- "socket_addr" "$REPO_ROOT/crates" 2>/dev/null | \
   grep -v "^[[:space:]]*//" | grep -q .; then
    echo "FAIL: 'socket_addr' appears in non-comment Rust code in crates/"
    grep -r --include="*.rs" -- "socket_addr" "$REPO_ROOT/crates" | grep -v "^[[:space:]]*//" || true
    RESULTS+=("FAIL: socket_addr in Rust code")
    FAIL=1
else
    RESULTS+=("PASS: socket_addr absent from Rust code (crates/)")
fi

# ── EphemeralEnvelope must not implement these traits ────────────────────────
check_absent_rs "impl Serialize for Ephemeral"
check_absent_rs "impl Debug for Ephemeral"
check_absent_rs "impl Display for Ephemeral"
check_absent_rs "impl Clone for Ephemeral"

# ── TOML configuration checks ─────────────────────────────────────────────────
check_absent_toml "regulated_disclosure_enabled"
check_absent_toml "priority_fees_enabled"
check_absent_toml "validator_fee_revenue_enabled"
check_absent_toml "monetary_governance_enabled"
check_absent_toml "oracle_supply_inputs_enabled"

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
