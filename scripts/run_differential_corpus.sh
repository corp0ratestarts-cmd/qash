#!/usr/bin/env bash
# Runs the canonical test vector corpus under two LLVM optimisation levels
# and diffs the state-root outputs. Pass: all roots match.
#
# Usage:
#   ./scripts/run_differential_corpus.sh              # both levels
#   ./scripts/run_differential_corpus.sh --opt0-only  # only opt-level=0
#
# Exit code: 0 = all roots identical, 1 = divergence detected.
set -euo pipefail

PASS=true

run_roots() {
    local level="$1"
    RUSTFLAGS="-C opt-level=${level}" \
        cargo test -p qash-consensus --no-default-features \
          state_root_canonical_seq_print -- --nocapture 2>/dev/null \
        | grep 'CANONICAL_STATE_ROOT' \
        || true
}

echo "=== opt-level=0 ==="
ROOTS_0=$(run_roots 0)
echo "$ROOTS_0"

if [[ "${1:-}" == "--opt0-only" ]]; then
    echo "opt0-only mode — skipping opt-level=3"
    exit 0
fi

echo ""
echo "=== opt-level=3 ==="
ROOTS_3=$(run_roots 3)
echo "$ROOTS_3"

echo ""
echo "=== Differential check ==="
if [ "$ROOTS_0" = "$ROOTS_3" ]; then
    echo "PASS: state roots identical across opt-level=0 and opt-level=3"
else
    echo "FAIL: state roots diverge across opt levels"
    diff <(echo "$ROOTS_0") <(echo "$ROOTS_3") || true
    PASS=false
fi

$PASS
