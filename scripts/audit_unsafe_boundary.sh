#!/usr/bin/env bash
# audit_unsafe_boundary.sh — Phase 4 of the pre-genesis full-repo audit.
#
# Enforces the unsafe boundary policy:
#
#   Domain A (crates/consensus/src/):
#     qash-consensus has #![forbid(unsafe_code)]. Any unsafe hit exits 1
#     unconditionally. SAFETY comments and exception entries do not override.
#
#   Domain B (crates/pal/src/, crates/address/src/, model/src/, src/):
#     Any unsafe block or function without a preceding // SAFETY: comment
#     (within 5 lines) AND without an entry in docs/audit/unsafe_exceptions.md
#     → advisory finding requiring triage.
#
# Also runs `cargo geiger --all-features` for a count summary (advisory).
#
# Status:
#   Domain A — Blocking: exit 1 on any unsafe hit.
#   Domain B — Advisory: exit 0, findings listed in report.
set -euo pipefail

OUTPUT_DIR="artifacts/audit"
OUTPUT_FILE="$OUTPUT_DIR/unsafe_boundary.md"
mkdir -p "$OUTPUT_DIR"

COMMIT_SHA=$(git rev-parse HEAD)
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

DOMAIN_A_DIR="crates/consensus/src"
DOMAIN_B_DIRS=("crates/pal/src" "crates/address/src" "model/src" "src")
EXCEPTIONS_FILE="docs/audit/unsafe_exceptions.md"

FAIL=0
DOMAIN_A_HITS=()
DOMAIN_B_MISSING_SAFETY=()
DOMAIN_B_WITH_SAFETY=()

# Precise unsafe pattern — skips forbid/deny attribute lines.
UNSAFE_PATTERN='unsafe[[:space:]]*(\{|fn[[:space:]]|impl[[:space:]]|trait[[:space:]]|extern[[:space:]])'

# ── Check if a file:line has a // SAFETY: comment within the 5 preceding lines ─
has_safety_comment() {
  local file="$1"
  local lineno="$2"
  local start=$(( lineno - 5 ))
  [ "$start" -lt 1 ] && start=1
  # Read lines start..(lineno-1) looking for // SAFETY:
  awk -v s="$start" -v e="$(( lineno - 1 ))" \
    'NR >= s && NR <= e && /\/\/ SAFETY:/ { found=1 } END { exit !found }' \
    "$file" 2>/dev/null
}

# ── Check if file:line has an entry in the exceptions register ─────────────────
has_exception_entry() {
  local file="$1"
  local lineno="$2"
  if [ ! -f "$EXCEPTIONS_FILE" ]; then
    return 1
  fi
  # Look for the file path referenced in the exceptions register
  grep -qF "$file" "$EXCEPTIONS_FILE" 2>/dev/null
}

# ── Domain A scan ─────────────────────────────────────────────────────────────
if [ -d "$DOMAIN_A_DIR" ]; then
  echo "Scanning Domain A ($DOMAIN_A_DIR) — unconditional blocking..."
  while IFS= read -r file; do
    while IFS=: read -r _ lineno line; do
      # Skip lines that are the forbid/deny attribute itself
      if echo "$line" | grep -qE 'forbid\(unsafe_code\)|deny\(unsafe_code\)'; then
        continue
      fi
      # Skip comment lines
      if echo "$line" | grep -qE '^\s*//'; then
        continue
      fi
      DOMAIN_A_HITS+=("$file:$lineno: $line")
      FAIL=1
    done < <(grep -nP "$UNSAFE_PATTERN" "$file" 2>/dev/null || true)
  done < <(find "$DOMAIN_A_DIR" -name '*.rs' 2>/dev/null)
else
  echo "Warning: Domain A directory '$DOMAIN_A_DIR' not found." >&2
fi

# ── Domain B scan ─────────────────────────────────────────────────────────────
for domain_dir in "${DOMAIN_B_DIRS[@]}"; do
  if [ -d "$domain_dir" ]; then
    echo "Scanning Domain B ($domain_dir) — advisory..."
    while IFS= read -r file; do
      while IFS=: read -r _ lineno line; do
        # Skip forbid/deny attribute lines
        if echo "$line" | grep -qE 'forbid\(unsafe_code\)|deny\(unsafe_code\)'; then
          continue
        fi
        # Skip comment lines
        if echo "$line" | grep -qE '^\s*//'; then
          continue
        fi
        # Check for SAFETY comment or exception entry
        if has_safety_comment "$file" "$lineno" || has_exception_entry "$file" "$lineno"; then
          DOMAIN_B_WITH_SAFETY+=("$file:$lineno: $line")
        else
          DOMAIN_B_MISSING_SAFETY+=("$file:$lineno: $line")
        fi
      done < <(grep -nP "$UNSAFE_PATTERN" "$file" 2>/dev/null || true)
    done < <(find "$domain_dir" -name '*.rs' 2>/dev/null)
  fi
done

# ── cargo geiger (advisory count summary) ────────────────────────────────────
GEIGER_OUTPUT=""
if command -v cargo >/dev/null 2>&1; then
  echo "Running cargo geiger (advisory)..."
  GEIGER_OUTPUT=$(cargo geiger --all-features 2>&1 || echo "(cargo geiger failed or not installed — advisory only)")
fi

# ── Emit report ───────────────────────────────────────────────────────────────
{
  echo "# Unsafe Boundary Audit"
  echo ""
  echo "**Commit:** \`$COMMIT_SHA\`  "
  echo "**Timestamp:** $TIMESTAMP  "
  echo "**Domain A status:** $([ "$FAIL" -eq 0 ] && echo "✅ PASS" || echo "❌ FAIL — ${#DOMAIN_A_HITS[@]} violation(s)")"
  echo "**Domain B missing SAFETY comment:** ${#DOMAIN_B_MISSING_SAFETY[@]} advisory finding(s)"
  echo "**Domain B with SAFETY comment:** ${#DOMAIN_B_WITH_SAFETY[@]} compliant site(s)"
  echo ""
  echo "## Policy"
  echo ""
  echo "**Domain A:** \`qash-consensus\` has \`#![forbid(unsafe_code)]\`. Any \`unsafe\` hit"
  echo "exits 1 unconditionally. SAFETY comments and exception entries do not override"
  echo "— Domain A forbids unsafe absolutely."
  echo ""
  echo "**Domain B:** Any \`unsafe\` block or function without a preceding \`// SAFETY:\`"
  echo "comment (within 5 lines) AND without an entry in \`$EXCEPTIONS_FILE\`"
  echo "→ advisory finding requiring triage before genesis-lock."
  echo ""
  echo "**unsafe detection pattern** (precise — skips \`forbid\`/\`deny\` attribute lines):"
  echo "\`\`\`"
  echo "unsafe\\s*(\\{|fn\\s|impl\\s|trait\\s|extern\\s)"
  echo "\`\`\`"
  echo ""
  echo "## Domain A results (blocking)"
  echo ""
  echo "- **Directory:** \`$DOMAIN_A_DIR\`"
  echo ""
  if [ "${#DOMAIN_A_HITS[@]}" -eq 0 ]; then
    echo "✅ No unsafe found — consistent with \`#![forbid(unsafe_code)]\`."
  else
    echo "❌ **${#DOMAIN_A_HITS[@]} unsafe hit(s) — each is a blocking failure:**"
    echo ""
    for v in "${DOMAIN_A_HITS[@]}"; do
      echo "- \`$v\`"
    done
  fi
  echo ""
  echo "## Domain B results (advisory)"
  echo ""
  if [ "${#DOMAIN_B_MISSING_SAFETY[@]}" -eq 0 ] && [ "${#DOMAIN_B_WITH_SAFETY[@]}" -eq 0 ]; then
    echo "✅ No unsafe found in Domain B."
  else
    if [ "${#DOMAIN_B_WITH_SAFETY[@]}" -gt 0 ]; then
      echo "### Compliant unsafe sites (have // SAFETY: comment or exception entry)"
      echo ""
      for v in "${DOMAIN_B_WITH_SAFETY[@]}"; do
        echo "- ✅ \`$v\`"
      done
      echo ""
    fi
    if [ "${#DOMAIN_B_MISSING_SAFETY[@]}" -gt 0 ]; then
      echo "### Missing SAFETY comment or exception entry (advisory — triage required)"
      echo ""
      for v in "${DOMAIN_B_MISSING_SAFETY[@]}"; do
        echo "- ⚠️ \`$v\`"
      done
      echo ""
      echo "Each of the above requires either:"
      echo "1. A \`// SAFETY: <explanation>\` comment within 5 lines immediately before the block, OR"
      echo "2. An entry in \`$EXCEPTIONS_FILE\` with owner sign-off."
    fi
  fi
  echo ""
  echo "## cargo geiger count summary (advisory)"
  echo ""
  echo "\`\`\`"
  echo "$GEIGER_OUTPUT"
  echo "\`\`\`"
  echo ""
  echo "## Verdict"
  echo ""
  if [ "$FAIL" -eq 0 ]; then
    echo "**PASS** — Domain A is clean. Domain B has ${#DOMAIN_B_MISSING_SAFETY[@]} advisory finding(s) requiring triage."
  else
    echo "**FAIL** — ${#DOMAIN_A_HITS[@]} Domain A violation(s). Domain A forbids unsafe absolutely."
  fi
} > "$OUTPUT_FILE"

echo ""
echo "Unsafe boundary audit complete."
echo "  Domain A hits: ${#DOMAIN_A_HITS[@]}"
echo "  Domain B missing SAFETY: ${#DOMAIN_B_MISSING_SAFETY[@]}"
echo "  Domain B compliant: ${#DOMAIN_B_WITH_SAFETY[@]}"
echo "  Report: $OUTPUT_FILE"

if [ "$FAIL" -ne 0 ]; then
  echo "  BLOCKING: ${#DOMAIN_A_HITS[@]} Domain A violation(s)." >&2
  exit 1
fi
echo "  PASS"
