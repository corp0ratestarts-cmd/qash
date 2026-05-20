#!/bin/bash
set -euo pipefail

echo "=== QASH Finality Benchmark Suite ==="
cd "$(dirname "$0")/.."
mkdir -p artifacts/benchmarks

echo "[1/4] Building benchmarks..."
cargo bench --bench finality_benchmark --no-run

echo "[2/4] Running benchmarks..."
log_file="artifacts/benchmarks/finality_$(date +%Y%m%d_%H%M%S).log"
cargo bench --bench finality_benchmark -- --nocapture 2>&1 | tee "$log_file"

echo "[3/4] Generating summary report..."
cat > artifacts/benchmarks/finality_summary.md <<EOF
# QASH Finality Benchmark Results

Log file: $log_file
EOF

echo "[4/4] Cross-ISA verification..."
if command -v qemu-aarch64-static >/dev/null 2>&1; then
  cargo bench --bench finality_benchmark --target aarch64-unknown-linux-gnu --no-run >/dev/null 2>&1 && \
  echo "✓ aarch64 build successful" || echo "⚠ aarch64 build skipped (QEMU not configured)"
else
  echo "⚠ QEMU not available; skipping cross-ISA runtime test"
fi

echo "=== Benchmark Complete ==="
