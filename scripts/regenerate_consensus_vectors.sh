#!/usr/bin/env bash
set -euo pipefail
QASH_REGENERATE_VECTORS=1 cargo test -p qash-consensus regenerate_vectors_when_requested -- --exact --nocapture
cargo test -p qash-consensus deterministic_vectors_match_golden -- --exact
