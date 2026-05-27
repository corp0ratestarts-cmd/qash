#!/usr/bin/env bash
# audit_liveness_loops.sh — Phase 5 of the pre-genesis full-repo audit.
#
# Finds loop constructs and checks whether they have an obvious termination,
# bounded progress evidence, or an explicit // INTENTIONAL_LOOP: annotation.
#
# Loop patterns:
#   loop\s*{    while\s+true    while\s+let    while <condition>
#
# Termination evidence (checks next 20 lines):
#   break | return | recv\s*\( | sleep\s*\( | yield | \.await | Halt:: | // INTENTIONAL_LOOP:
#
# Bounded progress evidence for non-`while true` loops:
#   if an identifier used in the while-condition is mutated in the next 20 lines
#   using +=, -=, *=, /=, %=, >>=, <<=, or =, the loop is classified as SAFE.
#   This captures deterministic bounded loops such as:
#     while a_bits != 0 { ... a_bits >>= 1; }
#     while pos + 128 <= input.len() { ... pos += 128; }
#
# Status:
#   Domain A — Blocking: WARN (no obvious termination/progress) → exit 1.
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

# ── Loop analysis helpers ─────────────────────────────────────────────────────
termination_found() {
  local file="$1"
  local lineno="$2"
  local end=$(( lineno + 20 ))
  local total_lines
  total_lines=$(wc -l < "$file")
  [ "$end" -gt "$total_lines" ] && end="$total_lines"

  awk -v s="$lineno" -v e="$end" '
    NR > s && NR <= e && /break|return|recv[[:space:]]*\(|sleep[[:space:]]*\(|yield|\.await|Halt::|\/\/[[:space:]]*INTENTIONAL_LOOP:/ { found=1 }
    END { exit !found }
  ' "$file" 2>/dev/null
}

condition_progress_found() {
  local file="$1"
  local lineno="$2"
  local loop_line="$3"

  # `loop {` and `while true` have no condition variable; require explicit
  # termination evidence or INTENTIONAL_LOOP annotation instead.
  if echo "$loop_line" | grep -qP '^.*\bloop[[:space:]]*\{|\bwhile[[:space:]]+true\b'; then
    return 1
  fi

  local condition
  condition=$(echo "$loop_line" | sed -E 's/^.*while[[:space:]]+//; s/[[:space:]]*\{.*$//')
  [ -n "$condition" ] || return 1

  local end=$(( lineno + 20 ))
  local total_lines
  total_lines=$(wc -l < "$file")
  [ "$end" -gt "$total_lines" ] && end="$total_lines"

  local body
  body=$(sed -n "$(( lineno + 1 )),${end}p" "$file")

  # Extract likely condition variables and ignore Rust keywords/types/functions
  # that are not mutable progress variables.
  local identifiers
  identifiers=$(echo "$condition" | grep -oE '[A-Za-z_][A-Za-z0-9_]*' | grep -vE '^(while|let|mut|true|false|Some|None|Ok|Err|len|is_empty|as|u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize)$' | sort -u || true)
  [ -n "$identifiers" ] || return 1

  while IFS= read -r ident; do
    [ -z "$ident" ] && continue
    if echo "$body" | grep -qP "(^|[^A-Za-z0-9_])${ident}([^A-Za-z0-9_]|$).*([+\-*/%<>]{1,2}=|=[^=])"; then
      return 0
    fi
  done <<< "$identifiers"

  return 1
}

# ── Check a loop construct at a given file:lineno ─────────────────────────────
# Returns 0 (SAFE) or 1 (WARN — no termination/progress found)
check_loop_termination() {
  local file="$1"
  local lineno="$2"
  local loop_line
  loop_line=$(sed -n "${lineno}p" "$file")

  if termination_found "$file" "$lineno"; then
    return 0
  fi

  if condition_progress_found "$file" "$lineno" "$loop_line"; then
    return 0
  fi

  return 1
}

# ── Domain A ──────────────────────────────────────────────────────────────────
if [ -d "$DOMAIN_A_DIR" ]; then
  echo "Scanning Domain A ($DOMAIN_A_DIR) — blocking..."
  while IFS= read -r file; do
    while IFS=: read -r _ lineno line; do
      [ -z "$lineno" ] && continue
      if check_loop_termination "$file" "$lineno"; then
        DOMAIN_A_SAFE+=("$file:$lineno: $line")
      else
        DOMAIN_A_WARN+=("$file:$lineno: $line")
        FAIL=1
      fi
    done < <(grep -nP 'loop[[:space:]]*\{|while[[:space:]]+true|while[[:space:]]+let|while[[:space:]]+[^\{]+\{' "$file" 2>/dev/null || true)
  done < <(find "$DOMAIN_A_DIR" -name '*.rs' 2>/dev/null)
fi

# ── Domain B ──────────────────────────────────────────────────────────────────
for domain_dir in "${DOMAIN_B_DIRS[@]}"; do
  if [ -d "$domain_dir" ]; then
    echo "Scanning Domain B ($domain_dir) — advisory..."
    while IFS= read -r file; do
      while IFS=: read -r _ lineno line; do
        [ -z "$lineno" ] && continue
        if check_loop_termination "$file" "$lineno"; then
          DOMAIN_B_SAFE+=("$file:$lineno: $line")
        else
          DOMAIN_B_WARN+=("$file:$lineno: $line")
        fi
      done < <(grep -nP 'loop[[:space:]]*\{|while[[:space:]]+true|while[[:space:]]+let|while[[:space:]]+[^\{]+\{' "$file" 2>/dev/null || true)
    done < <(find "$domain_dir" -name '*.rs' 2>/dev/null)
  fi
done

# ── Emit report ───────────────────────────────────────────────────────────────
{
  echo "# Liveness Loop Scan"
  echo ""
  echo "**Commit:** \`$COMMIT_SHA\`  "
  echo "**Timestamp:** $TIMESTAMP  "
  echo "**Domain A status:** $([ "$FAIL" -eq 0 ] && echo "✅ PASS" || echo "❌ FAIL — ${#DOMAIN_A_WARN[@]} unclassified loop(s)")"
  echo "**Domain A safe loops:** ${#DOMAIN_A_SAFE[@]}"
  echo "**Domain B unclassified (advisory):** ${#DOMAIN_B_WARN[@]}"
  echo "**Domain B safe loops:** ${#DOMAIN_B_SAFE[@]}"
  echo ""
  echo "## Loop patterns detected"
  echo ""
  echo "\`\`\`"
  echo "loop\\s*{    while\\s+true    while\\s+let    while <condition>"
  echo "\`\`\`"
  echo ""
  echo "## Termination/progress evidence (next 20 lines checked)"
  echo ""
  echo "\`\`\`"
  echo "break | return | recv\\s*( | sleep\\s*( | yield | \\.await | Halt:: | // INTENTIONAL_LOOP:"
  echo "condition-variable mutation using +=, -=, *=, /=, %=, >>=, <<=, or ="
  echo "\`\`\`"
  echo ""
  echo "**SAFE** — has an obvious termination signal, bounded progress evidence, or explicit \`// INTENTIONAL_LOOP:\` comment.  "
  echo "**WARN** — no obvious termination/progress found in next 20 lines."
  echo ""
  echo "## Domain A results (blocking)"
  echo ""
  if [ "${#DOMAIN_A_WARN[@]}" -eq 0 ] && [ "${#DOMAIN_A_SAFE[@]}" -eq 0 ]; then
    echo "✅ No loop constructs found in Domain A."
  else
    if [ "${#DOMAIN_A_SAFE[@]}" -gt 0 ]; then
      echo "### SAFE loops (${#DOMAIN_A_SAFE[@]})"
      echo ""
      for v in "${DOMAIN_A_SAFE[@]}"; do
        echo "- ✅ \`$v\`"
      done
      echo ""
    fi
    if [ "${#DOMAIN_A_WARN[@]}" -gt 0 ]; then
      echo "### WARN loops — no termination/progress found (${#DOMAIN_A_WARN[@]}) — BLOCKING"
      echo ""
      for v in "${DOMAIN_A_WARN[@]}"; do
        echo "- ❌ \`$v\`"
      done
      echo ""
      echo "**Resolution:** Add a \`break\`, \`return\`, \`.await\`, \`Halt::\`,"
      echo "bounded condition-variable mutation, or \`// INTENTIONAL_LOOP: <reason>\`"
      echo "comment within 20 lines of each loop."
    fi
  fi
  echo ""
  echo "## Domain B results (advisory)"
  echo ""
  if [ "${#DOMAIN_B_WARN[@]}" -eq 0 ] && [ "${#DOMAIN_B_SAFE[@]}" -eq 0 ]; then
    echo "✅ No loop constructs found in Domain B."
  else
    if [ "${#DOMAIN_B_SAFE[@]}" -gt 0 ]; then
      echo "### SAFE loops (${#DOMAIN_B_SAFE[@]})"
      echo ""
      for v in "${DOMAIN_B_SAFE[@]}"; do
        echo "- ✅ \`$v\`"
      done
      echo ""
    fi
    if [ "${#DOMAIN_B_WARN[@]}" -gt 0 ]; then
      echo "### WARN loops — no termination/progress found (${#DOMAIN_B_WARN[@]}) — advisory"
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
    echo "**PASS** — all Domain A loops have obvious termination/progress. Domain B has ${#DOMAIN_B_WARN[@]} advisory finding(s)."
  else
    echo "**FAIL** — ${#DOMAIN_A_WARN[@]} Domain A loop(s) with no obvious termination/progress. Each must be"
    echo "annotated with \`// INTENTIONAL_LOOP: <reason>\` or given an explicit termination/progress signal."
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
  echo "  BLOCKING: ${#DOMAIN_A_WARN[@]} Domain A loop(s) without termination/progress." >&2
  exit 1
fi
echo "  PASS"