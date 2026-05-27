#!/usr/bin/env bash
# audit_claim_boundary.sh — Phase 9 of the pre-genesis full-repo audit.
#
# Scans all .md/.toml/.txt files tracked by git for prohibited phrases that
# constitute claim overreach. Exits 1 on any unallowlisted match.
#
# Status: Blocking — exit 1 on any violation.
#
# Allowlist marker:
#   <!-- claim-boundary-allow: <reason> -->
#   Suppresses that line AND the immediately following line only.
#
# Excluded from scan (these files list prohibited examples, not live claims):
#   docs/mvp/claims_register.md
#   docs/audit/**
#   docs/platforms/**
#   docs/release/**
#
# docs/funding/ and docs/compliance/ are NOT excluded — grant-facing and
# compliance-facing docs are exactly where overclaims are most dangerous.
set -euo pipefail

OUTPUT_DIR="artifacts/audit"
OUTPUT_FILE="$OUTPUT_DIR/claim_boundary.md"
mkdir -p "$OUTPUT_DIR"

COMMIT_SHA=$(git rev-parse HEAD)
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

FAIL=0
VIOLATIONS=()

# ── Prohibited phrases (case-insensitive) ─────────────────────────────────────
# These are unacceptable compliance or capability overclaims.
PROHIBITED_PHRASES=(
  'GDPR compliant'
  'FIPS validated'
  'FIPS certified'
  'NSA approved'
  'military certified'
  'NATO certified'
  'FedRAMP authoris'
  'Common Criteria certified'
  'CMMC compliant'
  'quantum secure'
  'production-ready'
  'mainnet-ready'
  'financial infrastructure'
  'payment system'
  '\btoken\b'
  '\bcustody\b'
  'settlement layer'
)

# ── Forbidden platform overclaims (outside docs/platforms/) ───────────────────
PLATFORM_OVERCLAIMS=(
  'supports all platforms'
  'runs on all'
  'MUSA support'
  'CUDA support'
  'ROCm support'
  'HSM support'
  'TPM support'
  'smartcard support'
  'TEE support'
  'full RTOS support'
)

# ── Build the file list ───────────────────────────────────────────────────────
# Scan .md, .toml, .txt files tracked by git, excluding the allowlist directories.
mapfile -t SCAN_FILES < <(
  git ls-files '*.md' '*.toml' '*.txt' | grep -v \
    -e '^docs/mvp/claims_register\.md$' \
    -e '^docs/audit/' \
    -e '^docs/platforms/' \
    -e '^docs/release/'
)

# ── Scanner function ──────────────────────────────────────────────────────────
# Scans a file for a pattern (case-insensitive).
# Respects the allowlist marker: a line containing the marker suppresses
# itself and the immediately following line only.
scan_file_for_pattern() {
  local file="$1"
  local pattern="$2"
  local -i lineno=0
  local -i skip_next=0
  local violation_found=0

  while IFS= read -r line || [ -n "$line" ]; do
    lineno=$(( lineno + 1 ))

    # If previous line was an allowlist marker, skip this line
    if [ "$skip_next" -eq 1 ]; then
      skip_next=0
      continue
    fi

    # If this line is an allowlist marker, skip it and set flag for next line
    if echo "$line" | grep -qF '<!-- claim-boundary-allow:'; then
      skip_next=1
      continue
    fi

    # Check for the pattern (case-insensitive)
    if echo "$line" | grep -qiP "$pattern"; then
      echo "  VIOLATION: $file:$lineno: $line"
      VIOLATIONS+=("$file:$lineno: $pattern")
      violation_found=1
    fi
  done < "$file"

  return $violation_found
}

# ── Scan for prohibited phrases ───────────────────────────────────────────────
echo "Scanning ${#SCAN_FILES[@]} files for prohibited phrases..."
for file in "${SCAN_FILES[@]}"; do
  [ -f "$file" ] || continue
  for phrase in "${PROHIBITED_PHRASES[@]}"; do
    if ! scan_file_for_pattern "$file" "$phrase" 2>/dev/null; then
      FAIL=1
    fi
  done
done

# ── Scan for platform overclaims (broader file set, excluding docs/platforms/) ─
echo "Scanning for platform overclaims (outside docs/platforms/)..."
mapfile -t PLATFORM_SCAN_FILES < <(
  git ls-files '*.md' '*.toml' '*.txt' | grep -v \
    -e '^docs/mvp/claims_register\.md$' \
    -e '^docs/audit/' \
    -e '^docs/platforms/' \
    -e '^docs/release/'
)

for file in "${PLATFORM_SCAN_FILES[@]}"; do
  [ -f "$file" ] || continue
  for phrase in "${PLATFORM_OVERCLAIMS[@]}"; do
    if ! scan_file_for_pattern "$file" "$phrase" 2>/dev/null; then
      FAIL=1
    fi
  done
done

# ── Emit report ───────────────────────────────────────────────────────────────
{
  echo "# Claim Boundary Scan"
  echo ""
  echo "**Commit:** \`$COMMIT_SHA\`  "
  echo "**Timestamp:** $TIMESTAMP  "
  echo "**Status:** $([ "$FAIL" -eq 0 ] && echo "✅ PASS — no violations" || echo "❌ FAIL — violations found")"
  echo ""
  echo "## Files scanned"
  echo ""
  echo "- **General scan:** ${#SCAN_FILES[@]} files (\`.md\`, \`.toml\`, \`.txt\` tracked by git, excluding exempt directories)"
  echo "- **Excluded:** \`docs/mvp/claims_register.md\`, \`docs/audit/\`, \`docs/platforms/\`, \`docs/release/\`"
  echo "- **NOT excluded:** \`docs/funding/\`, \`docs/compliance/\` (grant/compliance-facing docs are high-risk)"
  echo ""
  echo "## Prohibited phrases"
  echo ""
  echo "| Phrase | Status |"
  echo "|--------|--------|"
  for phrase in "${PROHIBITED_PHRASES[@]}"; do
    echo "| \`$phrase\` | enforced |"
  done
  echo ""
  echo "## Platform overclaims (outside \`docs/platforms/\`)"
  echo ""
  echo "| Phrase | Status |"
  echo "|--------|--------|"
  for phrase in "${PLATFORM_OVERCLAIMS[@]}"; do
    echo "| \`$phrase\` | enforced |"
  done
  echo ""
  if [ "${#VIOLATIONS[@]}" -gt 0 ]; then
    echo "## Violations found"
    echo ""
    for v in "${VIOLATIONS[@]}"; do
      echo "- \`$v\`"
    done
    echo ""
  fi
  echo "## Allowlist marker"
  echo ""
  echo "A line containing \`<!-- claim-boundary-allow: <reason> -->\` suppresses"
  echo "that line and the **immediately following line only**. No broader suppression."
  echo ""
  echo "## Verdict"
  echo ""
  if [ "$FAIL" -eq 0 ]; then
    echo "**PASS** — all scanned files are within the claim boundary."
  else
    echo "**FAIL** — ${#VIOLATIONS[@]} violation(s) found. Each must be removed,"
    echo "corrected, or explicitly allowlisted with justification."
  fi
} > "$OUTPUT_FILE"

echo ""
echo "Claim boundary scan complete."
echo "  Report: $OUTPUT_FILE"
if [ "$FAIL" -ne 0 ]; then
  echo "  BLOCKING: ${#VIOLATIONS[@]} violation(s) — see report for details." >&2
  exit 1
fi
echo "  PASS: no violations found."
