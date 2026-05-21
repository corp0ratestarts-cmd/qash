#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo-kani >/dev/null 2>&1 && ! command -v kani >/dev/null 2>&1; then
  echo "Kani is not installed. Install it with: cargo install kani-verifier" >&2
  exit 127
fi

cargo kani -p qash-consensus --tests --harness tx1_project_divergence_never_increases --harness tx1_project_divergence_rejects_excess_delta
