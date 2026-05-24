#!/usr/bin/env bash
set -euo pipefail

BANNED_CRATES=(
  "zmij"
  "serde_core"
)

for crate in "${BANNED_CRATES[@]}"; do
  if grep -q "name = \"${crate}\"" Cargo.lock; then
    echo "error: banned crate detected in Cargo.lock: ${crate}"
    exit 1
  fi
done

echo "OK: no banned crates detected in Cargo.lock"
