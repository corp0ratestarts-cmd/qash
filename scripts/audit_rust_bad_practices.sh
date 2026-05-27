#!/usr/bin/env bash
# audit_rust_bad_practices.sh — Phase 2 of the pre-genesis full-repo audit.
#
# Scans for bad Rust practices in Domain A and Domain B source.
#
# Status:
#   Domain A (crates/consensus/src/) — Blocking: exit 1 on any hit.
#   Domain B (crates/pal/src/, crates/address/src/, model/src/, src/) — Advisory:
#            counts only; exit 0, violations listed in report.
#
# unsafe detection pattern (precise — avoids false-positives on attribute lines
# and comment lines):
#   unsafe\s*(\{|fn\s|impl\s|trait\s|extern\s)
#
# Test stripping: reuses the awk filter from check_domain_a_tripwires.sh to
# exclude lines inside #[test] / #[cfg(test)] blocks before scanning.
set -euo pipefail

OUTPUT_DIR="artifacts/audit"
OUTPUT_FILE="$OUTPUT_DIR/rust_bad_practices.md"
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
# Strips lines inside #[test] fns and #[cfg(test)] mods before pattern matching.
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

# ── Pattern definitions ───────────────────────────────────────────────────────
# unsafe: precise pattern that skips forbid/deny attribute lines and comments.
# The strip_tests filter already removes comment lines, but we add an extra
# guard for forbid(unsafe_code) and deny(unsafe_code) attribute lines.
UNSAFE_PATTERN='unsafe[[:space:]]*(\{|fn[[:space:]]|impl[[:space:]]|trait[[:space:]]|extern[[:space:]])'

# Patterns that must not appear in Domain A production code.
# Whitespace-tolerant (e.g. unwrap( or unwrap ().
PATTERNS=(
  'unwrap[[:space:]]*\('
  'expect[[:space:]]*\('
  'panic![[:space:]]*\('
  'unreachable![[:space:]]*\('
  'todo![[:space:]]*\('
  'unimplemented![[:space:]]*\('
  'get_unchecked[[:space:]]*\('
  'from_utf8_unchecked[[:space:]]*\('
  'MaybeUninit'
  'mem::zeroed'
  'mem::transmute'
  'static[[:space:]]+mut[[:space:]]'
  'Ordering::Relaxed'
  'thread::sleep'
  'SystemTime|Instant|OsRng|thread_rng|getrandom'
  'std::fs::|std::net::|std::env::'
  'tokio::'
  'loop[[:space:]]*\{'
  'while[[:space:]]+true'
  'as[[:space:]]+\*'
)

# Pattern labels (parallel to PATTERNS array)
PATTERN_LABELS=(
  'unwrap()'
  'expect()'
  'panic!()'
  'unreachable!()'
  'todo!()'
  'unimplemented!()'
  'get_unchecked()'
  'from_utf8_unchecked()'
  'MaybeUninit'
  'mem::zeroed'
  'mem::transmute'
  'static mut'
  'Ordering::Relaxed'
  'thread::sleep'
  'SystemTime/Instant/OsRng/getrandom'
  'std::fs::/std::net::/std::env::'
  'tokio::'
  'loop {}'
  'while true'
  'as *ptr cast'
)

# ── Scan function ─────────────────────────────────────────────────────────────
# scan_for_pattern <stripped_output> <pattern> <label> <domain>
# Returns findings via stdout; sets FAIL=1 on Domain A hit.
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
  echo "Scanning Domain A ($DOMAIN_A_DIR)..."
  STRIPPED_A=$(strip_tests "$DOMAIN_A_DIR")

  # Unsafe scan (with extra guard: skip forbid/deny attribute lines)
  UNSAFE_FINDINGS=$(echo "$STRIPPED_A" | grep -P "$UNSAFE_PATTERN" | \
    grep -v 'forbid(unsafe_code)' | grep -v 'deny(unsafe_code)' || true)
  if [ -n "$UNSAFE_FINDINGS" ]; then
    while IFS= read -r line; do
      [ -z "$line" ] && continue
      DOMAIN_A_VIOLATIONS+=("[unsafe] $line")
      FAIL=1
    done <<< "$UNSAFE_FINDINGS"
  fi

  # Other patterns
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

    # Unsafe scan
    UNSAFE_FINDINGS=$(echo "$STRIPPED_B" | grep -P "$UNSAFE_PATTERN" | \
      grep -v 'forbid(unsafe_code)' | grep -v 'deny(unsafe_code)' || true)
    if [ -n "$UNSAFE_FINDINGS" ]; then
      while IFS= read -r line; do
        [ -z "$line" ] && continue
        DOMAIN_B_ADVISORY+=("[unsafe] $line")
      done <<< "$UNSAFE_FINDINGS"
    fi

    # Other patterns
    for i in "${!PATTERNS[@]}"; do
      scan_for_pattern "$STRIPPED_B" "${PATTERNS[$i]}" "${PATTERN_LABELS[$i]}" "domain-b"
    done
  fi
done

# ── Emit report ───────────────────────────────────────────────────────────────
{
  echo "# Rust Bad Practices Scan"
  echo ""
  echo "**Commit:** \`$COMMIT_SHA\`  "
  echo "**Timestamp:** $TIMESTAMP  "
  echo "**Domain A status:** $([ "$FAIL" -eq 0 ] && echo "✅ PASS" || echo "❌ FAIL — ${#DOMAIN_A_VIOLATIONS[@]} violation(s)")"
  echo "**Domain B advisory count:** ${#DOMAIN_B_ADVISORY[@]} finding(s)"
  echo ""
  echo "## Patterns scanned"
  echo ""
  echo "**unsafe detection** (precise — skips \`forbid\`/\`deny\` attribute lines and comments):"
  echo "\`\`\`"
  echo "unsafe\\s*(\\{|fn\\s|impl\\s|trait\\s|extern\\s)"
  echo "\`\`\`"
  echo ""
  echo "**Additional patterns** (whitespace-tolerant):"
  echo ""
  for label in "${PATTERN_LABELS[@]}"; do
    echo "- \`$label\`"
  done
  echo ""
  echo "## Domain A results (blocking)"
  echo ""
  echo "- **Directory:** \`$DOMAIN_A_DIR\`"
  echo "- **Test code:** stripped via awk filter (from \`check_domain_a_tripwires.sh\`)"
  echo ""
  if [ "${#DOMAIN_A_VIOLATIONS[@]}" -eq 0 ]; then
    echo "✅ No violations found."
  else
    echo "❌ **${#DOMAIN_A_VIOLATIONS[@]} violation(s) — each is a blocking failure:**"
    echo ""
    for v in "${DOMAIN_A_VIOLATIONS[@]}"; do
      echo "- \`$v\`"
    done
  fi
  echo ""
  echo "## Domain B results (advisory)"
  echo ""
  echo "- **Directories:** ${DOMAIN_B_DIRS[*]}"
  echo "- **Test code:** stripped via awk filter"
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
    echo "**PASS** — Domain A is clean. Domain B has ${#DOMAIN_B_ADVISORY[@]} advisory finding(s) requiring triage."
  else
    echo "**FAIL** — ${#DOMAIN_A_VIOLATIONS[@]} Domain A violation(s). Must be fixed before genesis-lock."
  fi
} > "$OUTPUT_FILE"

echo ""
echo "Rust bad practices scan complete."
echo "  Domain A violations: ${#DOMAIN_A_VIOLATIONS[@]}"
echo "  Domain B advisories: ${#DOMAIN_B_ADVISORY[@]}"
echo "  Report: $OUTPUT_FILE"

if [ "$FAIL" -ne 0 ]; then
  echo "  BLOCKING: ${#DOMAIN_A_VIOLATIONS[@]} Domain A violation(s)." >&2
  exit 1
fi
echo "  PASS"
