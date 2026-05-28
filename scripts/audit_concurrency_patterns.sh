#!/usr/bin/env bash
# audit_concurrency_patterns.sh — Phase 7 of the pre-genesis full-repo audit.
#
# Scans for concurrency patterns and flags potential lock-across-await issues.
# Status: Advisory — exit 0 always.
#
# Patterns scanned:
#   Mutex, RwLock, Arc<Mutex, Atomic[A-Za-z]*, Ordering::Relaxed,
#   spawn\s*\(, \.await, thread::sleep
#
# Also flags lock-across-await: lines that acquire a lock (Mutex/RwLock .lock())
# followed by .await within the same lexical block (within 20 lines).
set -euo pipefail

OUTPUT_DIR="artifacts/audit"
OUTPUT_FILE="$OUTPUT_DIR/concurrency_patterns.md"
mkdir -p "$OUTPUT_DIR"

COMMIT_SHA=$(git rev-parse HEAD)
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

ALL_DIRS=("crates/consensus/src" "crates/pal/src" "crates/address/src" "model/src" "src")

declare -A PATTERN_HITS

PATTERNS=(
  'Mutex'
  'RwLock'
  'Arc<Mutex'
  'Atomic[A-Za-z]+'
  'Ordering::Relaxed'
  'spawn[[:space:]]*\('
  '\.await'
  'thread::sleep'
)

PATTERN_LABELS=(
  'Mutex'
  'RwLock'
  'Arc<Mutex>'
  'Atomic* types'
  'Ordering::Relaxed'
  'spawn()'
  '.await'
  'thread::sleep'
)

# Initialise hit arrays
for label in "${PATTERN_LABELS[@]}"; do
  PATTERN_HITS["$label"]=""
done

LOCK_ACROSS_AWAIT=()

# ── Scan all source directories ───────────────────────────────────────────────
for dir in "${ALL_DIRS[@]}"; do
  [ -d "$dir" ] || continue
  echo "Scanning $dir..."

  while IFS= read -r file; do
    for i in "${!PATTERNS[@]}"; do
      pattern="${PATTERNS[$i]}"
      label="${PATTERN_LABELS[$i]}"
      while IFS= read -r hit; do
        [ -z "$hit" ] && continue
        PATTERN_HITS["$label"]+="$hit"$'\n'
      done < <(grep -nP "$pattern" "$file" 2>/dev/null | sed "s|^|$file:|" || true)
    done

    # Lock-across-await detection:
    # Find lines where a Mutex/RwLock .lock() is called, then check if
    # .await appears within the next 20 lines (potential held-lock-across-await).
    while IFS=: read -r lineno line; do
      [ -z "$lineno" ] && continue
      end=$(( lineno + 20 ))
      total=$(wc -l < "$file")
      [ "$end" -gt "$total" ] && end="$total"
      if awk -v s="$lineno" -v e="$end" \
           'NR > s && NR <= e && /\.await/ { found=1 } END { exit !found }' \
           "$file" 2>/dev/null; then
        LOCK_ACROSS_AWAIT+=("$file:$lineno: $line")
      fi
    done < <(grep -nP '\.(lock|read|write)\(\)' "$file" 2>/dev/null || true)
  done < <(find "$dir" -name '*.rs' 2>/dev/null)
done

# ── Emit report ───────────────────────────────────────────────────────────────
{
  echo "# Concurrency Pattern Audit"
  echo ""
  echo "**Commit:** \`$COMMIT_SHA\`  "
  echo "**Timestamp:** $TIMESTAMP  "
  echo "**Status:** Advisory — exit 0 always. Findings require triage before genesis-lock."
  echo ""
  echo "## Pattern summary"
  echo ""
  echo "| Pattern | Hit count |"
  echo "|---------|-----------|"
  for label in "${PATTERN_LABELS[@]}"; do
    count=$(echo "${PATTERN_HITS[$label]}" | grep -c . || true)
    echo "| \`$label\` | $count |"
  done
  echo ""
  echo "## Lock-across-await candidates"
  echo ""
  if [ "${#LOCK_ACROSS_AWAIT[@]}" -eq 0 ]; then
    echo "✅ No lock-across-await patterns detected."
  else
    echo "⚠️ **${#LOCK_ACROSS_AWAIT[@]} potential lock-across-await site(s):**"
    echo ""
    echo "These are sites where a lock acquisition (\`.lock()\`, \`.read()\`, \`.write()\`)"
    echo "is followed by \`.await\` within 20 lines — a potential deadlock if the"
    echo "executor parks the task while holding the lock."
    echo ""
    for v in "${LOCK_ACROSS_AWAIT[@]}"; do
      echo "- ⚠️ \`$v\`"
    done
  fi
  echo ""
  echo "## Detailed findings by pattern"
  echo ""
  for label in "${PATTERN_LABELS[@]}"; do
    hits="${PATTERN_HITS[$label]}"
    count=$(echo "$hits" | grep -c . || true)
    echo "### \`$label\` ($count hits)"
    echo ""
    if [ "$count" -eq 0 ]; then
      echo "_No hits._"
    else
      while IFS= read -r h; do
        [ -z "$h" ] && continue
        echo "- \`$h\`"
      done <<< "$hits"
    fi
    echo ""
  done
  echo "## Verdict"
  echo ""
  echo "**Advisory only** — this scan always exits 0. All findings require triage"
  echo "and a documented decision in \`docs/audit/dependency_risk_register.md\`"
  echo "before genesis-lock. Lock-across-await candidates should be reviewed"
  echo "against the \`await_holding_lock\` Clippy lint (see Phase 3)."
} > "$OUTPUT_FILE"

echo ""
echo "Concurrency pattern audit complete (advisory)."
echo "  Report: $OUTPUT_FILE"
# Always exit 0 — advisory only
exit 0
