#!/usr/bin/env bash
# audit_file_inventory.sh — Phase 1 of the pre-genesis full-repo audit.
#
# Classifies every file tracked by `git ls-files` into one of eleven classes
# and emits:
#   artifacts/audit/file_inventory.json
#   artifacts/audit/file_inventory.md
#
# Status: Blocking (completion check — script must finish and produce output).
# Exit code: 0 on success; 1 on any error.
set -euo pipefail

OUTPUT_DIR="artifacts/audit"
JSON_OUT="$OUTPUT_DIR/file_inventory.json"
MD_OUT="$OUTPUT_DIR/file_inventory.md"

mkdir -p "$OUTPUT_DIR"

COMMIT_SHA=$(git rev-parse HEAD)
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# ── Classification rules (first match wins) ──────────────────────────────────
classify_file() {
  local path="$1"
  case "$path" in
    crates/consensus/src/*)        echo "domain-a" ;;
    crates/pal/src/*|crates/address/src/*|model/src/*) echo "domain-b" ;;
    src/*)                         echo "binary" ;;
    proofs/*)                      echo "proofs" ;;
    .github/workflows/*)           echo "ci-workflow" ;;
    scripts/*)                     echo "scripts" ;;
    docs/*|spec/*|tla/*|patents/*) echo "docs" ;;
    tests/*|fuzz/*)                echo "tests" ;;
    artifacts/*)                   echo "artifacts" ;;
    Cargo.toml|Cargo.lock|GENESIS_CONSTANTS.toml|rust-toolchain.toml|.cargo/*|deny.toml|.clippy.toml|clippy.toml|rustfmt.toml|.rustfmt.toml|.gitignore|.gitattributes) echo "config" ;;
    *)                             echo "other" ;;
  esac
}

# ── Build per-class file lists ────────────────────────────────────────────────
declare -A CLASS_FILES
declare -A CLASS_COUNTS

CLASSES=(domain-a domain-b binary proofs ci-workflow scripts docs tests artifacts config other)
for cls in "${CLASSES[@]}"; do
  CLASS_FILES[$cls]=""
  CLASS_COUNTS[$cls]=0
done

TOTAL=0
while IFS= read -r file; do
  cls=$(classify_file "$file")
  CLASS_FILES[$cls]+="$file"$'\n'
  CLASS_COUNTS[$cls]=$(( CLASS_COUNTS[$cls] + 1 ))
  TOTAL=$(( TOTAL + 1 ))
done < <(git ls-files)

# ── Emit JSON ─────────────────────────────────────────────────────────────────
{
  echo "{"
  echo "  \"commit\": \"$COMMIT_SHA\","
  echo "  \"timestamp\": \"$TIMESTAMP\","
  echo "  \"total_files\": $TOTAL,"
  echo "  \"classes\": {"
  first_cls=1
  for cls in "${CLASSES[@]}"; do
    if [ $first_cls -eq 0 ]; then echo "    ,"; fi
    first_cls=0
    echo "    \"$cls\": {"
    echo "      \"count\": ${CLASS_COUNTS[$cls]},"
    echo "      \"files\": ["
    first_file=1
    while IFS= read -r f; do
      [ -z "$f" ] && continue
      if [ $first_file -eq 0 ]; then echo "        ,"; fi
      first_file=0
      # Escape JSON string
      escaped=$(printf '%s' "$f" | sed 's/\\/\\\\/g; s/"/\\"/g')
      echo "        \"$escaped\""
    done <<< "${CLASS_FILES[$cls]}"
    echo "      ]"
    echo "    }"
  done
  echo "  }"
  echo "}"
} > "$JSON_OUT"

# ── Emit Markdown ─────────────────────────────────────────────────────────────
{
  echo "# File Inventory"
  echo ""
  echo "**Commit:** \`$COMMIT_SHA\`  "
  echo "**Timestamp:** $TIMESTAMP  "
  echo "**Total tracked files:** $TOTAL"
  echo ""
  echo "## Summary"
  echo ""
  echo "| Class | Count |"
  echo "|-------|-------|"
  for cls in "${CLASSES[@]}"; do
    echo "| \`$cls\` | ${CLASS_COUNTS[$cls]} |"
  done
  echo ""
  echo "## Per-class file lists"
  echo ""
  for cls in "${CLASSES[@]}"; do
    echo "### \`$cls\` (${CLASS_COUNTS[$cls]} files)"
    echo ""
    if [ "${CLASS_COUNTS[$cls]}" -eq 0 ]; then
      echo "_No files._"
    else
      while IFS= read -r f; do
        [ -z "$f" ] && continue
        echo "- \`$f\`"
      done <<< "${CLASS_FILES[$cls]}"
    fi
    echo ""
  done
} > "$MD_OUT"

echo "File inventory complete: $TOTAL files classified across ${#CLASSES[@]} classes."
echo "  JSON: $JSON_OUT"
echo "  MD:   $MD_OUT"
