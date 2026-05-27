#!/usr/bin/env bash
# run_clippy_strict_audit.sh — Phase 3 of the pre-genesis full-repo audit.
#
# Runs cargo clippy with pedantic, nursery, and QASH-specific lints.
# Status: Advisory — exit 0 always. Individual lints are promoted to blocking
#         only after triage is documented in the dependency risk register.
#
# Output: artifacts/audit/strict_clippy.txt
set -euo pipefail

OUTPUT_DIR="artifacts/audit"
OUTPUT_FILE="$OUTPUT_DIR/strict_clippy.txt"
mkdir -p "$OUTPUT_DIR"

COMMIT_SHA=$(git rev-parse HEAD)
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

echo "=== Strict Clippy Audit (Advisory) ===" | tee "$OUTPUT_FILE"
echo "Commit:    $COMMIT_SHA" | tee -a "$OUTPUT_FILE"
echo "Timestamp: $TIMESTAMP" | tee -a "$OUTPUT_FILE"
echo "" | tee -a "$OUTPUT_FILE"

# ── Clippy invocation ─────────────────────────────────────────────────────────
# Pedantic + nursery + QASH-specific lints.
# No default features to mirror the consensus build profile.
# continue-on-error: Clippy warnings do not fail this script.
echo "Running cargo clippy --workspace --all-targets --no-default-features ..." | tee -a "$OUTPUT_FILE"
echo "" | tee -a "$OUTPUT_FILE"

cargo clippy \
  --workspace \
  --all-targets \
  --no-default-features \
  -- \
  -W clippy::pedantic \
  -W clippy::nursery \
  -W clippy::indexing_slicing \
  -W clippy::integer_arithmetic \
  -W clippy::cast_possible_truncation \
  -W clippy::cast_sign_loss \
  -W clippy::await_holding_lock \
  -W clippy::mutex_atomic \
  -W clippy::mutex_integer \
  2>&1 | tee -a "$OUTPUT_FILE" || true

echo "" | tee -a "$OUTPUT_FILE"
echo "=== Strict Clippy Audit Complete (advisory — exit 0) ===" | tee -a "$OUTPUT_FILE"
echo "Report: $OUTPUT_FILE"

# Always exit 0 — advisory only. Promote individual lints to blocking
# via the pre-genesis-full-repo-audit.yml workflow configuration after triage.
exit 0
