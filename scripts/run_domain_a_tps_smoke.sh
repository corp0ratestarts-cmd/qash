#!/usr/bin/env bash
# Run the Domain A TPS smoke model and archive a timestamped report.
#
# This is a quick capacity/bottleneck triage harness, not a final performance
# claim. It measures CPU-only Domain A advance_epoch throughput and prints a
# linear independent-shard capacity model.
set -euo pipefail

ITERS="${ITERS:-200}"
WARMUP="${WARMUP:-20}"
SHARDS="${SHARDS:-1,4,16,64}"
OUT_DIR="${OUT_DIR:-artifacts/benchmarks}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
RUSTC="$(rustc --version | awk '{print $2}')"
COMMIT="$(git rev-parse --short=12 HEAD)"
OUT_FILE="$OUT_DIR/${STAMP}-domain-a-tps-${COMMIT}-rust-${RUSTC}.txt"

mkdir -p "$OUT_DIR"

{
  echo "# Domain A TPS smoke report"
  echo "timestamp=$STAMP"
  echo "commit=$(git rev-parse HEAD)"
  echo "rustc=$(rustc --version --verbose | tr '\n' ';')"
  echo "iters=$ITERS"
  echo "warmup=$WARMUP"
  echo "shards=$SHARDS"
  echo ""
  cargo run -p qash-consensus --release --example domain_a_tps_smoke -- \
    --iters "$ITERS" \
    --warmup "$WARMUP" \
    --shards "$SHARDS"
} | tee "$OUT_FILE"

echo ""
echo "Domain A TPS report written to: $OUT_FILE"
