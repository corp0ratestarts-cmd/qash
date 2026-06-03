#!/usr/bin/env bash
# Profile Boundary Enforcement — QASH Umbrella Repo
#
# Verifies that the umbrella qash repo does not accidentally claim Pure QASH-only
# privacy properties. The umbrella is Regulated-capable by design; it MUST NOT
# carry release artifacts that assert no-Class-IV or no-disclosure-key as
# invariants of the umbrella codebase.
#
# See: docs/adr/ADR-015-pure-qash-repository-split.md
#      docs/spec/19_profile_taxonomy.md
#
# Exit 0 = boundary intact. Exit 1 = violation found.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FAIL=0

echo "=== QASH Profile Boundary Check ==="
echo "Repo: $REPO_ROOT"
echo ""

# ── Rule 1: pure-qash/ must not contain implementation artifacts ─────────────
# The pure-qash/ dir is allowed to contain only README.md (the pointer).
PQDIR="$REPO_ROOT/pure-qash"
if [[ -d "$PQDIR" ]]; then
    UNEXPECTED=$(find "$PQDIR" -type f ! -name "README.md" 2>/dev/null | head -5)
    if [[ -n "$UNEXPECTED" ]]; then
        echo "FAIL: pure-qash/ contains implementation files (should only have README.md):"
        echo "$UNEXPECTED"
        FAIL=1
    else
        echo "PASS: pure-qash/ contains only pointer README.md"
    fi
else
    echo "PASS: pure-qash/ directory absent"
fi

# ── Rule 2: Umbrella must not carry a pure-qash absence guard script ─────────
if [[ -f "$REPO_ROOT/scripts/check_pure_absence_guards.sh" ]]; then
    echo "FAIL: scripts/check_pure_absence_guards.sh found — belongs in pure-qash repo only"
    FAIL=1
else
    echo "PASS: No pure-qash absence guard in umbrella"
fi

# ── Rule 3: Release docs must not claim Pure QASH-only privacy ───────────────
# Allowed: discussing Pure QASH as a separate profile (taxonomy, ADR, roadmap)
# Disallowed: claiming the umbrella release has no-Class-IV as an invariant
PURE_ONLY_CLAIMS=(
    "no Class IV observer"
    "no disclosure key"
    "no_disclosure_key = true"
    "class_iv_enabled = false"
)
for claim in "${PURE_ONLY_CLAIMS[@]}"; do
    if grep -r --include="*.md" --include="*.toml" -l "$claim" \
       "$REPO_ROOT/docs/release" "$REPO_ROOT/GENESIS_CONSTANTS.toml" 2>/dev/null | \
       grep -v "profile_taxonomy\|ADR-015\|pure-qash" | grep -q .; then
        echo "FAIL: Release artifact claims Pure QASH-only property: '$claim'"
        FAIL=1
    fi
done
echo "PASS: Release artifacts do not claim Pure QASH-only privacy invariants"

# ── Rule 4: Profile taxonomy doc must exist ───────────────────────────────────
TAXONOMY="$REPO_ROOT/docs/spec/19_profile_taxonomy.md"
if [[ -f "$TAXONOMY" ]]; then
    echo "PASS: Profile taxonomy doc present"
else
    echo "FAIL: docs/spec/19_profile_taxonomy.md missing — required by ADR-015"
    FAIL=1
fi

# ── Rule 5: ADR-015 must be present ──────────────────────────────────────────
ADR015="$REPO_ROOT/docs/adr/ADR-015-pure-qash-repository-split.md"
if [[ -f "$ADR015" ]]; then
    echo "PASS: ADR-015 present"
else
    echo "FAIL: docs/adr/ADR-015-pure-qash-repository-split.md missing"
    FAIL=1
fi

echo ""
if [[ $FAIL -eq 0 ]]; then
    echo "Profile Boundary: ALL PASS"
    exit 0
else
    echo "Profile Boundary: FAILED"
    exit 1
fi
