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
# Excluded from scan:
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

# Contextual overclaim patterns, case-insensitive grep -P regexes.
# Broad protocol words are not blocked alone because QASH legitimately uses
# capability-token, non-custodial, no-custody, and boundary-disclaimer language.
PROHIBITED_PHRASES=(
  'GDPR[[:space:]-]+compliant'
  'FIPS[[:space:]-]+validat'
  'FIPS[[:space:]-]+certif'
  'NSA[[:space:]-]+approv'
  'military[[:space:]-]+certif'
  'NATO[[:space:]-]+certif'
  'FedRAMP[[:space:]-]+authoris'
  'Common[[:space:]-]+Criteria[[:space:]-]+certif'
  'CMMC[[:space:]-]+compliant'
  'quantum[[:space:]-]+secure'
  'production[[:space:]-]+ready'
  'mainnet[[:space:]-]+ready'
  'financial[[:space:]-]+infrastructure'
  'payment[[:space:]-]+system'
  'settlement[[:space:]-]+layer'
  'investment[[:space:]-]+token'
  'utility[[:space:]-]+token'
  'security[[:space:]-]+token'
  'governance[[:space:]-]+token'
  'token[[:space:]-]+sale'
  'asset[[:space:]-]+custody'
  'funds[[:space:]-]+custody'
  'customer[[:space:]-]+custody'
  'custodial[[:space:]-]+service'
  'custody[[:space:]-]+of[[:space:]-]+assets'
)

NEGATIVE_CONTEXT='(^|[^[:alnum:]_])(not|no|non|never|without|must not|do not|cannot|should not|prohibit|prohibited|forbidden|blocked|avoid|no claim of|not a claim of|is not|are not)[^\.\n]{0,160}'
EXAMPLE_CONTEXT='(^|[[:space:]>#*-])((the )?prohibited claims are|avoid[[:space:]]*\(claim boundary violations\)|blocked:|blocked claims?|prohibited profile behavior|must not:|must never|do not use blocked terms)'

PLATFORM_OVERCLAIMS=(
  'supports[[:space:]-]+all[[:space:]-]+platforms'
  'runs[[:space:]-]+on[[:space:]-]+all[[:space:]-]+platforms'
  'runs[[:space:]-]+on[[:space:]-]+every[[:space:]-]+platform'
  'runs[[:space:]-]+on[[:space:]-]+all[[:space:]-]+supported[[:space:]-]+platforms'
  'MUSA[[:space:]-]+support'
  'CUDA[[:space:]-]+support'
  'ROCm[[:space:]-]+support'
  'HSM[[:space:]-]+support'
  'TPM[[:space:]-]+support'
  'smartcard[[:space:]-]+support'
  'TEE[[:space:]-]+support'
  'full[[:space:]-]+RTOS[[:space:]-]+support'
)

mapfile -t SCAN_FILES < <(
  git ls-files '*.md' '*.toml' '*.txt' | grep -v \
    -e '^docs/mvp/claims_register\.md$' \
    -e '^docs/audit/' \
    -e '^docs/platforms/' \
    -e '^docs/release/'
)

line_has_allow_marker() {
  local line="$1"
  echo "$line" | grep -qF '<!-- claim-boundary-allow:'
}

line_enters_example_context() {
  local line="$1"
  echo "$line" | grep -qiP "$EXAMPLE_CONTEXT"
}

line_is_negative_context_for_pattern() {
  local line="$1"
  local pattern="$2"
  echo "$line" | grep -qiP "${NEGATIVE_CONTEXT}${pattern}"
}

scan_file_for_pattern() {
  local file="$1"
  local pattern="$2"
  local -i lineno=0
  local -i skip_next=0
  local -i example_context=0
  local violation_found=0

  while IFS= read -r line || [ -n "$line" ]; do
    lineno=$(( lineno + 1 ))

    if echo "$line" | grep -qP '^#{1,3}[[:space:]]+'; then
      example_context=0
    fi

    if line_enters_example_context "$line"; then
      example_context=40
      continue
    fi

    if [ "$skip_next" -eq 1 ]; then
      skip_next=0
      continue
    fi

    if line_has_allow_marker "$line"; then
      skip_next=1
      continue
    fi

    if echo "$line" | grep -qiP "$pattern"; then
      if [ "$example_context" -gt 0 ]; then
        example_context=$(( example_context - 1 ))
        continue
      fi
      if line_is_negative_context_for_pattern "$line" "$pattern"; then
        continue
      fi
      echo "  VIOLATION: $file:$lineno: $line"
      VIOLATIONS+=("$file:$lineno: $pattern")
      violation_found=1
    fi

    if [ "$example_context" -gt 0 ]; then
      example_context=$(( example_context - 1 ))
    fi
  done < "$file"

  return $violation_found
}

echo "Scanning ${#SCAN_FILES[@]} files for prohibited claim patterns..."
for file in "${SCAN_FILES[@]}"; do
  [ -f "$file" ] || continue
  for phrase in "${PROHIBITED_PHRASES[@]}"; do
    if ! scan_file_for_pattern "$file" "$phrase" 2>/dev/null; then
      FAIL=1
    fi
  done
done

echo "Scanning for platform overclaims outside docs/platforms/..."
for file in "${SCAN_FILES[@]}"; do
  [ -f "$file" ] || continue
  for phrase in "${PLATFORM_OVERCLAIMS[@]}"; do
    if ! scan_file_for_pattern "$file" "$phrase" 2>/dev/null; then
      FAIL=1
    fi
  done
done

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
  echo "- **NOT excluded:** \`docs/funding/\`, \`docs/compliance/\`"
  echo ""
  echo "## Pattern groups"
  echo ""
  echo "- Compliance/certification overclaim patterns: ${#PROHIBITED_PHRASES[@]}"
  echo "- Platform overclaim patterns: ${#PLATFORM_OVERCLAIMS[@]}"
  echo ""
  echo "## Suppression policy"
  echo ""
  echo "Clearly negative uses and explicit blocked/prohibited/avoid example sections are not treated as live claims."
  echo "The narrow allowlist marker remains available for one-off cases."
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