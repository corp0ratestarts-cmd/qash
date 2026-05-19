#!/usr/bin/env bash
# capture_proof_hashes.sh — Compile active Coq proofs and emit a hash manifest.
#
# Output format (stdout):
#   # coq-version: <coqc --version output, first line>
#   <sha256>  ./<path/to/file.vo>
#   ...
#
# Exit codes:
#   0  All proofs compile and manifest is written to stdout.
#   1  coqc not found, or one or more proofs fail to compile.
#
# Usage:
#   # Print manifest to stdout:
#   ./scripts/capture_proof_hashes.sh
#
#   # Commit manifest for current HEAD:
#   ./scripts/capture_proof_hashes.sh \
#     | tee proofs/artifact-index/proof-hashes-$(git rev-parse HEAD).txt
#   git add proofs/artifact-index/
#   git commit -m "ci: record proof hashes for $(git rev-parse --short HEAD)"
#
# Requirements: coqc on PATH, sha256sum (coreutils).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROOFS_DIR="$REPO_ROOT/proofs"

if ! command -v coqc &>/dev/null; then
    echo "error: coqc not found. Install Coq and ensure it is on PATH." >&2
    exit 1
fi

COQ_VERSION="$(coqc --version 2>&1 | head -1)"
echo "# coq-version: $COQ_VERSION"

# Compile in dependency order (mirrors the CI proofs job).
cd "$PROOFS_DIR"

compile() {
    local f="$1"
    if ! coqc -Q . QASH "$f" 2>/dev/null; then
        echo "error: coqc failed on $f" >&2
        exit 1
    fi
}

# Tier 1: no cross-QASH dependencies.
compile crypto_game_framework.v
compile util/list_inj.v
compile contractivity/lyapunov_stability.v

# Tier 2: depend on Tier 1 or stdlib only.
for f in \
    concat_injective.v \
    contractivity/encode_injectivity.v \
    contractivity/tx_perturbation_0.v \
    contractivity/tx1_score_decrement.v \
    contractivity/lyapunov_grace_convergence.v \
    lyapunov_decrease.v \
    safety/absorbing_halt.v \
    integration/th8_composition.v \
    cascade/cascade_health_bounded.v \
    cascade/cascade_determinism.v \
    cascade/cascade_collision_resistance.v \
    cascade/it_mac_forgery_bound.v \
    blinding/blinding_non_interference.v \
    model/Model.v; do
    compile "$f"
done

# Emit sorted hash manifest.
find . -name "*.vo" | sort | xargs sha256sum
