#!/usr/bin/env bash
# audit_liveness_loops.sh — Phase 5 of the pre-genesis full-repo audit.
#
# Finds loop constructs and checks whether they have an obvious termination,
# bounded condition, or an explicit // INTENTIONAL_LOOP: annotation.
#
# Blocking policy:
#   - `loop {`, `while true`, and `while let` require explicit termination evidence
#     or an INTENTIONAL_LOOP annotation.
#   - Ordinary `while <condition> {` loops are classified as bounded-condition loops
#     and reported as SAFE. They remain visible in the report for review.
#
# This gate is intended to catch accidental unbounded event/spin loops in Domain A,
# not to prove full termination of every arithmetic loop.
#
# Status:
#   Domain A — Blocking: WARN (unbounded-looking loop) → exit 1.
#   Domain B / scripts — Advisory: exit 0.
set -euo pipefail

OUTPUT_DIR="artifacts/audit"
OUTPUT_FILE="$OUTPUT_DIR/liveness_loops.md"
mkdir -p "$OUTPUT_DIR"

COMMIT_SHA=$(git rev-parse HEAD)
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

DOMAIN_A_DIR="crates/consensus/src"
DOMAIN_B_DIRS=("crates/pal/src" "crates/address/src" "model/src" "src")

FAIL=0
DOMAIN_A_WARN=()
DOMAIN_A_SAFE=()
DOMAIN_B_WARN=()
DOMAIN_B_SAFE=()

strip_comments_and_tests() {
  local dir="$1"
  find "$dir" -name '*.rs' 2>/dev/null | while IFS= read -r f; do
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

termination_found() {
  local file="$1"
  local lineno="$2"
  local end=$(( lineno + 20 ))
  local total_lines
  total_lines=$(wc -l < "$file")
  [ "$end" -gt "$total_lines" ] && end="$total_lines"

  awk -v s="$lineno" -v e="$end" '
    NR >= s && NR <= e && /break|return|recv[[:space:]]*\(|sleep[[:space:]]*\(|yield|\.await|Halt::|\/\/[[:space:]]*INTENTIONAL_LOOP:/ { found=1 }
    END { exit !found }
  ' "$file" 2>/dev/null
}

classify_loop_line() {
  local file="$1"
  local lineno="$2"
  local line="$3"

  if echo "$line" | grep -qP '\bloop[[:space:]]*\{|\bwhile[[:space:]]+true\b|\bwhile[[:space:]]+let\b'; then
    if termination_found "$file" "$lineno"; then
      echo "SAFE"
    else
      echo "WARN"
    fi
    return
  fi

  if echo "$line" | grep -qP '\bwhile[[:space:]]+[^\{]+\{'; then
    echo "SAFE"
    return
  fi

  echo "WARN"
}

scan_dir() {
  local dir="$1"
  local domain="$2"
  local stripped
  stripped=$(strip_comments_and_tests "$dir")

  while IFS=: read -r file lineno line; do
    [ -n "${file:-}" ] || continue
    [ -n "${lineno:-}" ] || continue

    local verdict
    verdict=$(classify_loop_line "$file" "$lineno" "$line")

    if [ "$domain" = "DOMAIN_A" ]; then
      if [ "$verdict" = "SAFE" ]; then
        DOMAIN_A_SAFE+=("$file:$lineno: $line")
      else
        DOMAIN_A_WARN+=("$file:$lineno: $line")
        FAIL=1
      fi
    else
      if [ "$verdict" = "SAFE" ]; then
        DOMAIN_B_SAFE+=("$file:$lineno: $line")
      else
        DOMAIN_B_WARN+=("$file:$lineno: $line")
      fi
    fi
  done < <(echo "$stripped" | grep -P '\bloop[[:space:]]*\{|\bwhile[[:space:]]+true\b|\bwhile[[:space:]]+let\b|\bwhile[[:space:]]+[^\{]+\{' || true)
}

# ── Domain A ──────────────────────────────────────────────────────────────────
if [ -d "$DOMAIN_A_DIR" ]; then
  echo "Scanning Domain A ($DOMAIN_A_DIR) — blocking..."
  scan_dir "$DOMAIN_A_DIR" "DOMAIN_A"
fi

# ── Domain B ──────────────────────────────────────────────────────────────────
for domain_dir in "${DOMAIN_B_DIRS[@]}"; do
  if [ -d "$domain_dir" ]; then
    echo "Scanning Domain B ($domain_dir) — advisory..."
    scan_dir "$domain_dir" "DOMAIN_B"
  fi
done

# ── Emit report ───────────────────────────────────────────────────────────────
{
  echo "# Liveness Loop Scan"
  echo ""
  echo "**Commit:** \`$COMMIT_SHA\`  "
  echo "**Timestamp:** $TIMESTAMP  "
  echo "**Domain A status:** $([ "$FAIL" -eq 0 ] && echo "✅ PASS" || echo "❌ FAIL — ${#DOMAIN_A_WARN[@]} unbounded-looking loop(s)")"
  echo "**Domain A safe/bounded loops:** ${#DOMAIN_A_SAFE[@]}"
  echo "**Domain B unclassified (advisory):** ${#DOMAIN_B_WARN[@]}"
  echo "**Domain B safe/bounded loops:** ${#DOMAIN_B_SAFE[@]}"
  echo ""
  echo "## Loop patterns detected"
  echo ""
  echo "\`\`\`"
  echo "loop\\s*{    while\\s+true    while\\s+let    while <condition>"
  echo "\`\`\`"
  echo ""
  echo "## Classification policy"
  echo ""
  echo "- \`loop {\`, \`while true\`, and \`while let\` require termination evidence or \`// INTENTIONAL_LOOP:\`."
  echo "- Ordinary \`while <condition> {\` loops are classified as bounded-condition loops and listed for review."
  echo "- Test functions/modules and Rust comments are stripped before scanning."
  echo ""
  echo "## Domain A results (blocking)"
  echo ""
  if [ "${#DOMAIN_A_WARN[@]}" -eq 0 ] && [ "${#DOMAIN_A_SAFE[@]}" -eq 0 ]; then
    echo "✅ No loop constructs found in Domain A."
  else
    if [ "${#DOMAIN_A_SAFE[@]}" -gt 0 ]; then
      echo "### SAFE / bounded loops (${#DOMAIN_A_SAFE[@]})"
      echo ""
      for v in "${DOMAIN_A_SAFE[@]}"; do
        echo "- ✅ \`$v\`"
      done
      echo ""
    fi
    if [ "${#DOMAIN_A_WARN[@]}" -gt 0 ]; then
      echo "### WARN loops — no termination evidence (${#DOMAIN_A_WARN[@]}) — BLOCKING"
      echo ""
      for v in "${DOMAIN_A_WARN[@]}"; do
        echo "- ❌ \`$v\`"
      done
      echo ""
      echo "**Resolution:** Add a \`break\`, \`return\`, \`.await\`, \`Halt::\`, or"
      echo "\`// INTENTIONAL_LOOP: <reason>\` comment within 20 lines of each loop."
    fi
  fi
  echo ""
  echo "## Domain B results (advisory)"
  echo ""
  if [ "${#DOMAIN_B_WARN[@]}" -eq 0 ] && [ "${#DOMAIN_B_SAFE[@]}" -eq 0 ]; then
    echo "✅ No loop constructs found in Domain B."
  else
    if [ "${#DOMAIN_B_SAFE[@]}" -gt 0 ]; then
      echo "### SAFE / bounded loops (${#DOMAIN_B_SAFE[@]})"
      echo ""
      for v in "${DOMAIN_B_SAFE[@]}"; do
        echo "- ✅ \`$v\`"
      done
      echo ""
    fi
    if [ "${#DOMAIN_B_WARN[@]}" -gt 0 ]; then
      echo "### WARN loops — no termination evidence (${#DOMAIN_B_WARN[@]}) — advisory"
      echo ""
      for v in "${DOMAIN_B_WARN[@]}"; do
        echo "- ⚠️ \`$v\`"
      done
    fi
  fi
  echo ""
  echo "## Verdict"
  echo ""
  if [ "$FAIL" -eq 0 ]; then
    echo "**PASS** — Domain A has no unbounded-looking loops. Bounded-condition loops are listed for review."
  else
    echo "**FAIL** — ${#DOMAIN_A_WARN[@]} Domain A loop(s) with no termination evidence."
  fi
} > "$OUTPUT_FILE"

echo ""
echo "Liveness loop scan complete."
echo "  Domain A WARN: ${#DOMAIN_A_WARN[@]}"
echo "  Domain A SAFE: ${#DOMAIN_A_SAFE[@]}"
echo "  Domain B WARN: ${#DOMAIN_B_WARN[@]}"
echo "  Domain B SAFE: ${#DOMAIN_B_SAFE[@]}"
echo "  Report: $OUTPUT_FILE"

if [ "$FAIL" -ne 0 ]; then
  echo "  BLOCKING: ${#DOMAIN_A_WARN[@]} Domain A loop(s) without termination evidence." >&2
  exit 1
fi
echo "  PASS"