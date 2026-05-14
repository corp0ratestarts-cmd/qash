#!/usr/bin/env bash
# Verify that the STRICT_PROOFS set in proofs/Makefile never shrinks across commits.
# A proof file may enter the set but cannot leave it (append-only invariant).
# Run from workspace root.
set -euo pipefail

MAKEFILE="proofs/Makefile"

if [[ ! -f "$MAKEFILE" ]]; then
  echo "ERROR: $MAKEFILE not found (run from workspace root)" >&2
  exit 1
fi

# Extract current STRICT_PROOFS list
extract_strict_proofs() {
  local ref="$1"  # git ref or empty string for working tree
  local content
  if [[ -z "$ref" ]]; then
    content=$(cat "$MAKEFILE")
  else
    content=$(git show "${ref}:${MAKEFILE}" 2>/dev/null) || { echo ""; return; }
  fi

  echo "$content" | awk '
    /^STRICT_PROOFS[ \t]*:=/ { in_block=1; next }
    in_block && /\\[ \t]*$/ { gsub(/\\[ \t]*$/, ""); gsub(/^[ \t]+/, ""); gsub(/[ \t]+$/, ""); if (length($0)) print; next }
    in_block { gsub(/^[ \t]+/, ""); gsub(/[ \t]+$/, ""); if (length($0)) print; in_block=0 }
  ' | grep '\.v$' | sort || true
}

# Get current HEAD and parent sets
HEAD_PROOFS=$(extract_strict_proofs "")
PREV_PROOFS=$(extract_strict_proofs "HEAD~1")

if [[ -z "$PREV_PROOFS" ]]; then
  echo "SKIP: no parent commit found (initial commit or shallow clone) — nothing to compare"
  exit 0
fi

echo "Previous STRICT_PROOFS:"
echo "$PREV_PROOFS" | sed 's/^/  /'
echo "Current STRICT_PROOFS:"
echo "$HEAD_PROOFS" | sed 's/^/  /'

errors=0
while IFS= read -r proof; do
  [[ -z "$proof" ]] && continue
  if ! echo "$HEAD_PROOFS" | grep -qF "$proof"; then
    echo "REMOVED: '$proof' was in STRICT_PROOFS but is no longer present" >&2
    errors=$((errors + 1))
  fi
done <<< "$PREV_PROOFS"

if [[ $errors -gt 0 ]]; then
  echo "FAIL: $errors proof(s) removed from STRICT_PROOFS — set is append-only" >&2
  exit 1
fi

added=$(comm -13 <(echo "$PREV_PROOFS") <(echo "$HEAD_PROOFS") | grep -v '^$' | wc -l)
echo "verify_strict_proofs_non_shrinking: OK (${added} added, 0 removed)"
