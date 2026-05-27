#!/usr/bin/env bash
# audit_panic_surface.sh — Phase 6 of the pre-genesis full-repo audit.
#
# Scans production code for panic/unwrap/assert patterns.
#
# Status:
#   Domain A (crates/consensus/src/) — Blocking: exit 1 on any hit.
#   Domain B (crates/pal/src/, crates/address/src/, model/src/, src/) — Advisory:
#            counts only; exit 0, violations listed in report.
#
# Test stripping: uses the same awk filter as check_domain_a_tripwires.sh
# and audit_rust_bad_practices.sh to exclude #[test]/#[cfg(test)] blocks.
set -euo pipefail

OUTPUT_DIR="artifacts/audit"
OUTPUT_FILE="$OUTPUT_DIR/panic_surface.md"
mkdir -p "$OUTPUT_DIR"

COMMIT_SHA=$(git rev-parse HEAD)
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

DOMAIN_A_DIR="crates/consensus/src"
DOMAIN_B_DIRS=("crates/pal/src" "crates/address/src" "model/src" "src")

FAIL=0
DOMAIN_A_VIOLATIONS=()
DOMAIN_B_ADVISORY=()

# ── awk test-stripping filter ─────────────────────────────────────────────────
# Reused from check_domain_a_tripwires.sh:34-55.
strip_tests() {
  local dir="$1"
  find "$dir" -name '*.rs' 2>/dev/null | while read -r f; do
    awk '
      /^[[:space:]]*#\[cfg\(test\)\]/ { in_test_mod = 1 }
      /^[[:space:]]*#\[test\]/ { skip_next_fn = 1 }
      skip_next_fn && /^[[:space:]]*(pub |pub\(crate\) |async )?fn / {
        in_test_fn = 1; skip_next_fn = 0; depth = 0
      }
      in_test_fn {
        for (i=1; i<=length($0); i++) {
          c = substr($0, i, 1)
          if (c == "{") depth++
          if (c == "}") { depth--; if (depth <= 0) { in_test_fn = 0; next } }
        }
        next
      }
      in_test_mod { next }
      /^[[:space:]]*\/\/[\/!]/ { next }
      /^[[:space:]]*\/\// { next }
      { print FILENAME ":" NR ":" $0 }
    ' FILENAME="$f" "$f"
  done
}

# ── Panic surface patterns ────────────────────────────────────────────────────
# Whitespace-tolerant — catches unwrap( and unwrap  ( etc.
PATTERNS=(
  'unwrap[[:space:]]*\('
  'expect[[:space:]]*\('
  'panic![[:space:]]*\('
  'assert![[:space:]]*\('
  'assert_eq![[:space:]]*\('
  'assert_ne![[:space:]]*\('
  'lock\(\)[[:space:]]*\.[[:space:]]*unwrap[[:space:]]*\('
  'join\(\)[[:space:]]*\.[[:space:]]*unwrap[[:space:]]*\('
)

PATTERN_LABELS=(
  'unwrap()'
  'expect()'
  'panic!()'
  'assert!()'
  'assert_eq!()'
  'assert_ne!()'
  'lock().unwrap()'
  'join().unwrap()'
)

# ── Scan function ─────────────────────────────────────────────────────────────
scan_for_pattern() {
  local stripped="$1"
  local pattern="$2"
  local label="$3"
  local domain="$4"
  local findings

  findings=$(echo "$stripped" | grep -P "$pattern" || true)
  if [ -n "$findings" ]; then
    while IFS= read -r line; do
      [ -z "$line" ] && continue
      if [ "$domain" = "domain-a" ]; then
        DOMAIN_A_VIOLATIONS+=("[$label] $line")
        FAIL=1
      else
        DOMAIN_B_ADVISORY+=("[$label] $line")
      fi
    done <<< "$findings"
  fi
}

# ── Domain A scan ─────────────────────────────────────────────────────────────
if [ -d "$DOMAIN_A_DIR" ]; then
  echo "Scanning Domain A ($DOMAIN_A_DIR) — blocking..."
  STRIPPED_A=$(strip_tests "$DOMAIN_A_DIR")

  for i in "${!PATTERNS[@]}"; do
    scan_for_pattern "$STRIPPED_A" "${PATTERNS[$i]}" "${PATTERN_LABELS[$i]}" "domain-a"
  done
else
  echo "Warning: Domain A directory '$DOMAIN_A_DIR' not found — skipping." >&2
fi

# ── Domain B scan (advisory) ──────────────────────────────────────────────────
for domain_dir in "${DOMAIN_B_DIRS[@]}"; do
  if [ -d "$domain_dir" ]; then
    echo "Scanning Domain B ($domain_dir) — advisory..."
    STRIPPED_B=$(strip_tests "$domain_dir")

    for i in "${!PATTERNS[@]}"; do
      scan_for_pattern "$STRIPPED_B" "${PATTERNS[$i]}" "${PATTERN_LABELS[$i]}" "domain-b"
    done
  fi
done

# ── Emit report ───────────────────────────────────────────────────────────────
{
  echo "# Panic Surface Scan"
  echo ""
  echo "**Commit:** \`$COMMIT_SHA\`  "
  echo "**Timestamp:** $TIMESTAMP  "
  echo "**Domain A status:** $([ "$FAIL" -eq 0 ] && echo "✅ PASS" || echo "❌ FAIL — ${#DOMAIN_A_VIOLATIONS[@]} violation(s)")"
  echo "**Domain B advisory count:** ${#DOMAIN_B_ADVISORY[@]} finding(s)"
  echo ""
  echo "## Patterns scanned (whitespace-tolerant)"
  echo ""
  for i in "${!PATTERN_LABELS[@]}"; do
    echo "- \`${PATTERN_LABELS[$i]}\` — \`${PATTERNS[$i]}\`"
  done
  echo ""
  echo "## Test stripping"
  echo ""
  echo "All scans strip \`#[test]\` functions and \`#[cfg(test)]\` modules via the"
  echo "awk filter from \`check_domain_a_tripwires.sh:34-55\`. Comment lines"
  echo "(\`//\`, \`///\`, \`//!\`) are also excluded."
  echo ""
  echo "## Domain A results (blocking)"
  echo ""
  echo "- **Directory:** \`$DOMAIN_A_DIR\`"
  echo ""
  if [ "${#DOMAIN_A_VIOLATIONS[@]}" -eq 0 ]; then
    echo "✅ No violations found."
  else
    echo "❌ **${#DOMAIN_A_VIOLATIONS[@]} violation(s) — blocking failures:**"
    echo ""
    for v in "${DOMAIN_A_VIOLATIONS[@]}"; do
      echo "- \`$v\`"
    done
  fi
  echo ""
  echo "## Domain B results (advisory)"
  echo ""
  echo "- **Directories:** ${DOMAIN_B_DIRS[*]}"
  echo ""
  if [ "${#DOMAIN_B_ADVISORY[@]}" -eq 0 ]; then
    echo "✅ No advisory findings."
  else
    echo "⚠️ **${#DOMAIN_B_ADVISORY[@]} advisory finding(s) — triage required before genesis-lock:**"
    echo ""
    for v in "${DOMAIN_B_ADVISORY[@]}"; do
      echo "- \`$v\`"
    done
  fi
  echo ""
  echo "## Verdict"
  echo ""
  if [ "$FAIL" -eq 0 ]; then
    echo "**PASS** — Domain A panic surface is clean. Domain B has ${#DOMAIN_B_ADVISORY[@]} advisory finding(s) requiring triage."
  else
    echo "**FAIL** — ${#DOMAIN_A_VIOLATIONS[@]} Domain A violation(s). Each panic/unwrap/assert in production consensus code must be removed before genesis-lock."
  fi
} > "$OUTPUT_FILE"

echo ""
echo "Panic surface scan complete."
echo "  Domain A violations: ${#DOMAIN_A_VIOLATIONS[@]}"
echo "  Domain B advisories: ${#DOMAIN_B_ADVISORY[@]}"
echo "  Report: $OUTPUT_FILE"

if [ "$FAIL" -ne 0 ]; then
  echo "  BLOCKING: ${#DOMAIN_A_VIOLATIONS[@]} Domain A violation(s)." >&2
  exit 1
fi
echo "  PASS"
