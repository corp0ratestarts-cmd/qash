#!/usr/bin/env bash
# generate_coq_vectors.sh — regenerate proofs/model/vectors.json from the Rust
# implementation and verify that the updated vectors still pass coq_model_parity.
#
# Usage:
#   bash scripts/generate_coq_vectors.sh [--update]
#
# Without --update: runs the generator and prints the new JSON to stdout;
#   also runs the parity test against the EXISTING vectors.json to verify no drift.
# With --update: overwrites proofs/model/vectors.json with the regenerated output
#   and runs coq_model_parity to confirm the new file passes.
#
# This script is the bridge between proofs/model/Model.v (Coq executable spec)
# and the Rust implementation.  Run it whenever advance_epoch() or any of the
# covered state-machine paths change.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VECTORS_JSON="$REPO_ROOT/proofs/model/vectors.json"

UPDATE=0
for arg in "$@"; do
    if [[ "$arg" == "--update" ]]; then
        UPDATE=1
    fi
done

cd "$REPO_ROOT"

echo "=== generate_coq_vectors.sh ==="
echo "Repo root: $REPO_ROOT"
echo ""

echo "--- Step 1: Build consensus crate (no-default-features) ---"
cargo build -p qash-consensus --no-default-features 2>&1

echo ""
echo "--- Step 2: Run vector generator ---"
GENERATED=$(cargo test -p qash-consensus --no-default-features \
    -- --nocapture --ignored gen_coq_vectors 2>/dev/null \
    | grep -A 10000 '^{' | head -n -0 || true)

if [[ -z "$GENERATED" ]]; then
    echo "ERROR: generator produced no output" >&2
    exit 1
fi

echo "$GENERATED"
echo ""

if [[ "$UPDATE" -eq 1 ]]; then
    echo "--- Step 3: Writing $VECTORS_JSON ---"
    echo "$GENERATED" > "$VECTORS_JSON"
    echo "Written."
else
    echo "--- Step 3: Skipped (pass --update to overwrite vectors.json) ---"
fi

echo ""
echo "--- Step 4: Run coq_model_parity regression test ---"
cargo test -p qash-consensus --no-default-features coq_model_parity 2>&1 \
    | grep -E "^test coq_model_parity|^test result"
echo ""
echo "=== Done ==="
