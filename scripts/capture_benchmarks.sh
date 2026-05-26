#!/usr/bin/env bash
# capture_benchmarks.sh — Run consensus benchmarks and archive results.
#
# Usage:
#   ./scripts/capture_benchmarks.sh [--filter PATTERN]
#
# Runs all Criterion benchmarks in qash-consensus and writes results to
# artifacts/benchmarks/ with commit hash, toolchain, and timestamp in the
# filename.
#
# The archived output is in Criterion's bencher format, suitable for
# regression tracking and Phase 2-R evidence capture.
#
# Examples:
#   ./scripts/capture_benchmarks.sh
#   ./scripts/capture_benchmarks.sh --filter phase2r
#   ./scripts/capture_benchmarks.sh --filter tx_heavy

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${REPO_ROOT}/artifacts/benchmarks"
FILTER=""

for arg in "$@"; do
    case "$arg" in
        --filter) shift; FILTER="$1"; shift ;;
        --filter=*) FILTER="${arg#--filter=}" ;;
        *) echo "Unknown argument: $arg" >&2; exit 1 ;;
    esac
done

cd "$REPO_ROOT"

# Gather metadata
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
TOOLCHAIN=$(rustc --version 2>/dev/null | awk '{print $2}' || echo "unknown")
TARGET=$(rustc -vV 2>/dev/null | grep '^host:' | awk '{print $2}' || echo "unknown")
TIMESTAMP=$(date -u +%Y%m%dT%H%M%SZ)
LABEL="${TIMESTAMP}-${COMMIT}-${TARGET}"

mkdir -p "$OUT_DIR"

OUTFILE="${OUT_DIR}/${LABEL}-epoch-transition.txt"

echo "=== QASH Consensus Benchmark Capture ==="
echo "commit:    $COMMIT"
echo "toolchain: $TOOLCHAIN"
echo "target:    $TARGET"
echo "output:    $OUTFILE"
[ -n "$FILTER" ] && echo "filter:    $FILTER"
echo

# Write metadata header
{
    echo "# QASH consensus benchmark capture"
    echo "# Commit:    $COMMIT"
    echo "# Toolchain: $TOOLCHAIN"
    echo "# Target:    $TARGET"
    echo "# Timestamp: $TIMESTAMP"
    [ -n "$FILTER" ] && echo "# Filter:    $FILTER"
    echo "# Command:   cargo bench -p qash-consensus -- --output-format bencher"
    echo "#"
} > "$OUTFILE"

# Run benchmarks
BENCH_ARGS="--output-format bencher"
if [ -n "$FILTER" ]; then
    cargo bench -p qash-consensus -- $BENCH_ARGS "$FILTER" 2>&1 | tee -a "$OUTFILE"
else
    cargo bench -p qash-consensus -- $BENCH_ARGS 2>&1 | tee -a "$OUTFILE"
fi

echo
echo "Benchmark results archived to: $OUTFILE"
echo
echo "Phase 2-R note: to use this as a baseline for optimisation claims,"
echo "compare this file against a post-Phase-2-R capture with the same filter."
echo "Results must show measurable improvement on 'phase2r_tx_heavy_advance'"
echo "while producing byte-identical state roots (verified by test-determinism CI)."
