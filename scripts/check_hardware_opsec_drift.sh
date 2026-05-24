#!/usr/bin/env bash
set -euo pipefail

patterns=(
  'trusted ceremony'
  'genesis ceremony'
  'network master key'
  'foundation.*master key'
  'YubiKey.*genesis'
  'YubiHSM.*genesis'
  'HSM.*genesis authority'
  'FIDO.*finality'
  'touch.*epoch'
  'hardware.*Domain A'
)

for pattern in "${patterns[@]}"; do
  if rg -i "$pattern" docs README.md ROADMAP.md PROJECT_STATUS.md; then
    echo "Potential trusted-ceremony or hardware-authority drift: $pattern" >&2
    exit 1
  fi
done

echo "Hardware OpSec anti-drift scan passed"
