#!/usr/bin/env bash
set -euo pipefail

BASE_REF="${GITHUB_BASE_SHA:-}"
if [[ -z "$BASE_REF" ]]; then
  echo "GITHUB_BASE_SHA not set; skipping HaltReason gate"
  exit 0
fi

if ! git cat-file -e "$BASE_REF^{commit}" 2>/dev/null; then
  echo "Base commit $BASE_REF unavailable; skipping HaltReason gate"
  exit 0
fi

changed_files=$(git diff --name-only "$BASE_REF"...HEAD)
if ! grep -q '^crates/consensus/src/transition.rs$' <<<"$changed_files"; then
  exit 0
fi

if ! git diff "$BASE_REF"...HEAD -- crates/consensus/src/transition.rs | grep -q "HaltReason"; then
  exit 0
fi

required=(
  "docs/spec/halt_taxonomy.md"
  "crates/consensus/tests/axioms.rs"
  "tests/vectors/vectors.v1.json"
)

for path in "${required[@]}"; do
  if ! grep -q "^${path}$" <<<"$changed_files"; then
    echo "HaltReason gate failure: expected update to ${path}"
    exit 1
  fi
done

echo "HaltReason gate passed"
