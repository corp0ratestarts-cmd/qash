#!/usr/bin/env bash
# audit_panic_surface.sh — Phase 6 of the pre-genesis full-repo audit.
#
# Scans production code for panic/unwrap/assert patterns.
#
# Status:
#   Domain A (crates/consensus/src/) — Blocking: exit 1 on panic/unwrap/expect/assert hits.
#   Domain A debug_assert*/debug_assert_eq*/debug_assert_ne* — Advisory only: compiled out of
#            release builds and retained as visibility for hardening review.
#   Domain B (crates/pal/src/, crates/address/src/, model/src/, src/) — Advisory:
#            counts only; exit 0, findings listed in report.
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
DOMAIN_A_DEBUG_ADVISORY=()
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
# Whitespace-tolerant. Assertion patterns use a non-word/line-start prefix so
# they do not accidentally match `debug_assert*` as `assert*`.
PATTERNS=(
  'unwrap[[:space:]]*\('
  'expect[[:space:]]*\('
  'panic![[:space:]]*\('
  '(^|[^[:alnum:]_])assert![[:space:]]*\('
  '(^|[^[:alnum:]_])assert_eq![[:space:]]*\('
  '(^|[^[:alnum:]_])assert_ne![[:space:]]*\('
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

DEBUG_PATTERNS=(
  'debug_assert![[:space:]]*\('
  'debug_assert_eq![[:space:]]*\('
  'debug_assert_ne![[:space:]]*\('
)

DEBUG_LABELS=(
  'debug_assert!()'
  'debug_assert_eq!()'
  'debug_assert_ne!()'
)

# ── Scan functions ────────────────────────────────────────────────────────────
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

scan_debug_for_domain_a() {
  local stripped="$1"
  local pattern="$2"
  local label="$3"
  local findings

  findings=$(echo "$stripped" | grep -P "$pattern" || true)
  if [ -n "$findings" ]; then
    while IFS= read -r line; do
      [ -z "$line" ] && continue
      DOMAIN_A_DEBUG_ADVISORY+=("[$label] $line")
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

  for i in "${!DEBUG_PATTERNS[@]}"; do
    scan_debug_for_domain_a "$STRIPPED_A" "${DEBUG_PATTERNS[$i]}" "${DEBUG_LABELS[$i]}"
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

    for i in "${!DEBUG_PATTERNS[@]}"; do
      scan_for_pattern "$STRIPPED_B" "${DEBUG_PATTERNS[$i]}" "${DEBUG_LABELS[$i]}" "domain-b"
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
  echo "**Domain A debug assertion advisory count:** ${#DOMAIN_A_DEBUG_ADVISORY[@]} finding(s)"
  echo "**Domain B advisory count:** ${#DOMAIN_B_ADVISORY[@]} finding(s)"
  echo ""
  echo "## Patterns scanned (whitespace-tolerant)"
  echo ""
  for i in "${!PATTERN_LABELS[@]}"; do
    echo "- \`${PATTERN_LABELS[$i]}\` — \`${PATTERNS[$i]}\`"
  done
  echo ""
  echo "## Debug assertion patterns (Domain A advisory only)"
  echo ""
  for i in "${!DEBUG_LABELS[@]}"; do
    echo "- \`${DEBUG_LABELS[$i]}\` — \`${DEBUG_PATTERNS[$i]}\`"
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
    echo "✅ No blocking violations found."
  else
    echo "❌ **${#DOMAIN_A_VIOLATIONS[@]} violation(s) — blocking failures:**"
    echo ""
    for v in "${DOMAIN_A_VIOLATIONS[@]}"; do
      echo "- \`$v\`"
    done
  fi
  echo ""
  echo "## Domain A debug assertions (advisory)"
  echo ""
  if [ "${#DOMAIN_A_DEBUG_ADVISORY[@]}" -eq 0 ]; then
    echo "✅ No debug assertions found."
  else
    echo "ℹ️ **${#DOMAIN_A_DEBUG_ADVISORY[@]} debug assertion(s) — advisory visibility only:**"
    echo ""
    for v in "${DOMAIN_A_DEBUG_ADVISORY[@]}"; do
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
    echo "**PASS** — Domain A blocking panic surface is clean. Domain A debug assertions and Domain B findings remain advisory triage items."
  else
    echo "**FAIL** — ${#DOMAIN_A_VIOLATIONS[@]} Domain A violation(s). Each panic/unwrap/assert in production consensus code must be removed before genesis-lock."
  fi
} > "$OUTPUT_FILE"

echo ""
echo "Panic surface scan complete."
echo "  Domain A blocking violations: ${#DOMAIN_A_VIOLATIONS[@]}"
echo "  Domain A debug assertion advisories: ${#DOMAIN_A_DEBUG_ADVISORY[@]}"
echo "  Domain B advisories: ${#DOMAIN_B_ADVISORY[@]}"
echo "  Report: $OUTPUT_FILE"

if [ "$FAIL" -ne 0 ]; then
  echo "  BLOCKING: ${#DOMAIN_A_VIOLATIONS[@]} Domain A violation(s)." >&2
  exit 1
fi
echo "  PASS"