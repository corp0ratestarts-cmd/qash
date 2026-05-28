#!/usr/bin/env bash
# build_pre_genesis_audit_report.sh — Phase 12 of the pre-genesis full-repo audit.
#
# Reads all artifacts/audit/*.md phase reports and emits a consolidated:
#   artifacts/audit/pre_genesis_full_repo_audit.md
#   artifacts/audit/pre_genesis_full_repo_audit.json
#
# Status: Runs on workflow_dispatch, push: main, and weekly schedule only.
# The blocking pass/fail verdict is the first line of the .md report.
set -euo pipefail

OUTPUT_DIR="artifacts/audit"
MD_OUT="$OUTPUT_DIR/pre_genesis_full_repo_audit.md"
JSON_OUT="$OUTPUT_DIR/pre_genesis_full_repo_audit.json"
mkdir -p "$OUTPUT_DIR"

COMMIT_SHA=$(git rev-parse HEAD)
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# ── Read phase reports ─────────────────────────────────────────────────────────
# For each phase, check if the report exists and whether it contains PASS/FAIL.

read_report_verdict() {
  local file="$1"
  if [ ! -f "$file" ]; then
    echo "MISSING"
    return
  fi
  if grep -qiE '\*\*FAIL\*\*|❌.*FAIL|^.*FAIL.*violation' "$file" 2>/dev/null; then
    echo "FAIL"
  elif grep -qiE '\*\*PASS\*\*|✅.*PASS' "$file" 2>/dev/null; then
    echo "PASS"
  else
    echo "UNKNOWN"
  fi
}

read_count() {
  local file="$1"
  local pattern="$2"
  if [ ! -f "$file" ]; then
    echo "N/A"
    return
  fi
  grep -oP "$pattern" "$file" 2>/dev/null | head -1 || echo "N/A"
}

# Phase reports
CLAIM_BOUNDARY_FILE="$OUTPUT_DIR/claim_boundary.md"
DOMAIN_BOUNDARY_FILE="$OUTPUT_DIR/domain_boundary_full.md"
RUST_BAD_FILE="$OUTPUT_DIR/rust_bad_practices.md"
PANIC_SURFACE_FILE="$OUTPUT_DIR/panic_surface.md"
UNSAFE_BOUNDARY_FILE="$OUTPUT_DIR/unsafe_boundary.md"
LIVENESS_LOOPS_FILE="$OUTPUT_DIR/liveness_loops.md"
CONCURRENCY_FILE="$OUTPUT_DIR/concurrency_patterns.md"
FILE_INVENTORY_FILE="$OUTPUT_DIR/file_inventory.md"
STRICT_CLIPPY_FILE="$OUTPUT_DIR/strict_clippy.txt"

CLAIM_VERDICT=$(read_report_verdict "$CLAIM_BOUNDARY_FILE")
DOMAIN_VERDICT=$(read_report_verdict "$DOMAIN_BOUNDARY_FILE")
RUST_VERDICT=$(read_report_verdict "$RUST_BAD_FILE")
PANIC_VERDICT=$(read_report_verdict "$PANIC_SURFACE_FILE")
UNSAFE_VERDICT=$(read_report_verdict "$UNSAFE_BOUNDARY_FILE")
LIVENESS_VERDICT=$(read_report_verdict "$LIVENESS_LOOPS_FILE")

# Determine overall blocking verdict
OVERALL_FAIL=0
for v in "$CLAIM_VERDICT" "$DOMAIN_VERDICT" "$RUST_VERDICT" "$PANIC_VERDICT" "$UNSAFE_VERDICT" "$LIVENESS_VERDICT"; do
  if [ "$v" = "FAIL" ] || [ "$v" = "MISSING" ]; then
    OVERALL_FAIL=1
    break
  fi
done

OVERALL_VERDICT=$([ "$OVERALL_FAIL" -eq 0 ] && echo "PASS" || echo "FAIL")

# ── File inventory counts ─────────────────────────────────────────────────────
TOTAL_FILES="N/A"
DOMAIN_A_COUNT="N/A"
DOMAIN_B_COUNT="N/A"
if [ -f "$FILE_INVENTORY_FILE" ]; then
  TOTAL_FILES=$(grep -oP '(?<=\*\*Total tracked files:\*\* )\d+' "$FILE_INVENTORY_FILE" 2>/dev/null | head -1 || echo "N/A")
  DOMAIN_A_COUNT=$(grep -A1 '`domain-a`' "$FILE_INVENTORY_FILE" 2>/dev/null | grep -oP '\d+' | head -1 || echo "N/A")
  DOMAIN_B_COUNT=$(grep -A1 '`domain-b`' "$FILE_INVENTORY_FILE" 2>/dev/null | grep -oP '\d+' | head -1 || echo "N/A")
fi

# ── Cargo workspace metadata ──────────────────────────────────────────────────
CARGO_METADATA_SUMMARY="N/A"
if command -v cargo >/dev/null 2>&1; then
  DEP_COUNT=$(cargo metadata --no-deps --format-version 1 2>/dev/null | \
    python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d.get('packages',[])))" 2>/dev/null || echo "N/A")
  CARGO_METADATA_SUMMARY="$DEP_COUNT workspace packages"
fi

# ── Proof status ─────────────────────────────────────────────────────────────
PROOF_STATUS="N/A"
if [ -d "proofs/" ]; then
  PROOF_COUNT=$(find proofs/ -name '*.v' 2>/dev/null | wc -l || echo "0")
  PROOF_STATUS="$PROOF_COUNT .v files present"
fi

# Open exception count
EXCEPTION_COUNT="0"
EXCEPTIONS_FILE="docs/audit/unsafe_exceptions.md"
if [ -f "$EXCEPTIONS_FILE" ]; then
  EXCEPTION_COUNT=$(grep -c '^### ' "$EXCEPTIONS_FILE" 2>/dev/null || echo "0")
fi

# ── Emit Markdown report ──────────────────────────────────────────────────────
{
  # Blocking verdict is the first line
  if [ "$OVERALL_VERDICT" = "PASS" ]; then
    echo "# ✅ Pre-Genesis Full-Repo Audit — PASS"
  else
    echo "# ❌ Pre-Genesis Full-Repo Audit — FAIL"
  fi
  echo ""
  echo "**Commit:** \`$COMMIT_SHA\`"
  echo "**Timestamp:** $TIMESTAMP"
  echo "**Overall verdict:** **$OVERALL_VERDICT**"
  echo ""
  echo "---"
  echo ""
  echo "## Blocking phase verdicts"
  echo ""
  echo "| Phase | Script | Verdict |"
  echo "|-------|--------|---------|"
  echo "| Phase 9 — Claim boundary | \`audit_claim_boundary.sh\` | $([ "$CLAIM_VERDICT" = "PASS" ] && echo "✅ PASS" || echo "❌ $CLAIM_VERDICT") |"
  echo "| Phase 10 — Domain A/B boundary | \`audit_domain_boundary_full.sh\` | $([ "$DOMAIN_VERDICT" = "PASS" ] && echo "✅ PASS" || echo "❌ $DOMAIN_VERDICT") |"
  echo "| Phase 2 — Rust bad practices | \`audit_rust_bad_practices.sh\` | $([ "$RUST_VERDICT" = "PASS" ] && echo "✅ PASS" || echo "❌ $RUST_VERDICT") |"
  echo "| Phase 6 — Panic surface | \`audit_panic_surface.sh\` | $([ "$PANIC_VERDICT" = "PASS" ] && echo "✅ PASS" || echo "❌ $PANIC_VERDICT") |"
  echo "| Phase 4 — Unsafe boundary | \`audit_unsafe_boundary.sh\` | $([ "$UNSAFE_VERDICT" = "PASS" ] && echo "✅ PASS" || echo "❌ $UNSAFE_VERDICT") |"
  echo "| Phase 5 — Liveness loops | \`audit_liveness_loops.sh\` | $([ "$LIVENESS_VERDICT" = "PASS" ] && echo "✅ PASS" || echo "❌ $LIVENESS_VERDICT") |"
  echo ""
  echo "## File inventory"
  echo ""
  echo "| Metric | Value |"
  echo "|--------|-------|"
  echo "| Total tracked files | $TOTAL_FILES |"
  echo "| Domain A files (\`crates/consensus/src/\`) | $DOMAIN_A_COUNT |"
  echo "| Domain B files (\`pal/address/model/src/\`) | $DOMAIN_B_COUNT |"
  echo "| Workspace packages | $CARGO_METADATA_SUMMARY |"
  echo ""
  echo "## Proof status"
  echo ""
  echo "| Metric | Value |"
  echo "|--------|-------|"
  echo "| Coq proof files | $PROOF_STATUS |"
  echo "| Open unsafe exceptions | $EXCEPTION_COUNT |"
  echo ""
  echo "## Advisory phase summaries"
  echo ""
  echo "### Phase 3 — Strict Clippy"
  if [ -f "$STRICT_CLIPPY_FILE" ]; then
    echo "_See \`artifacts/audit/strict_clippy.txt\` for full output._"
    CLIPPY_WARNINGS=$(grep -c '^warning' "$STRICT_CLIPPY_FILE" 2>/dev/null || echo "0")
    echo ""
    echo "Clippy warnings: $CLIPPY_WARNINGS"
  else
    echo "_Report not generated (runs on full audit trigger only)._"
  fi
  echo ""
  echo "### Phase 7 — Concurrency patterns"
  if [ -f "$CONCURRENCY_FILE" ]; then
    echo "_See \`artifacts/audit/concurrency_patterns.md\`._"
    LOCK_AWAIT=$(grep -oP '(?<=\*\*)\d+(?= potential lock-across-await)' "$CONCURRENCY_FILE" 2>/dev/null | head -1 || echo "0")
    echo ""
    echo "Lock-across-await candidates: $LOCK_AWAIT"
  else
    echo "_Report not generated._"
  fi
  echo ""
  echo "## Dependency risk"
  echo ""
  RISK_ENTRIES=$(awk '
    /^## Current entries/ { in_entries = 1; next }
    /^## / { in_entries = 0 }
    in_entries && /^### / { count++ }
    END { print count + 0 }
  ' "docs/audit/dependency_risk_register.md" 2>/dev/null || echo "0")
  echo "Open dependency risk entries: $RISK_ENTRIES"
  echo ""
  echo "_See \`docs/audit/dependency_risk_register.md\` for triage status._"
  echo ""
  echo "## Genesis-lock gate"
  echo ""
  if [ "$OVERALL_VERDICT" = "PASS" ]; then
    echo "✅ All blocking phases pass. This commit is eligible for genesis-lock"
    echo "subject to: dependency risk triage complete, all advisory findings"
    echo "triaged with documented decisions, and exception register reviewed."
  else
    echo "❌ One or more blocking phases failed. This commit is NOT eligible for"
    echo "genesis-lock. Resolve all FAIL verdicts above before proceeding."
  fi
  echo ""
  echo "---"
  echo ""
  echo "## Phase report index"
  echo ""
  for report in "$OUTPUT_DIR"/*.md; do
    [ -f "$report" ] || continue
    basename=$(basename "$report")
    [ "$basename" = "pre_genesis_full_repo_audit.md" ] && continue
    echo "- [\`$basename\`](./$basename)"
  done
} > "$MD_OUT"

# ── Emit JSON report ──────────────────────────────────────────────────────────
{
  echo "{"
  echo "  \"commit\": \"$COMMIT_SHA\","
  echo "  \"timestamp\": \"$TIMESTAMP\","
  echo "  \"overall_verdict\": \"$OVERALL_VERDICT\","
  echo "  \"blocking_phases\": {"
  echo "    \"claim_boundary\": \"$CLAIM_VERDICT\","
  echo "    \"domain_boundary_full\": \"$DOMAIN_VERDICT\","
  echo "    \"rust_bad_practices\": \"$RUST_VERDICT\","
  echo "    \"panic_surface\": \"$PANIC_VERDICT\","
  echo "    \"unsafe_boundary\": \"$UNSAFE_VERDICT\","
  echo "    \"liveness_loops\": \"$LIVENESS_VERDICT\""
  echo "  },"
  echo "  \"file_inventory\": {"
  echo "    \"total_files\": \"$TOTAL_FILES\","
  echo "    \"domain_a_files\": \"$DOMAIN_A_COUNT\","
  echo "    \"domain_b_files\": \"$DOMAIN_B_COUNT\""
  echo "  },"
  echo "  \"proof_status\": \"$PROOF_STATUS\","
  echo "  \"open_exception_count\": $EXCEPTION_COUNT,"
  echo "  \"dependency_risk_entries\": $RISK_ENTRIES"
  echo "}"
} > "$JSON_OUT"

echo ""
echo "Pre-genesis full-repo audit report built."
echo "  Verdict: $OVERALL_VERDICT"
echo "  MD:   $MD_OUT"
echo "  JSON: $JSON_OUT"

# The report builder itself exits 0 (the blocking jobs in CI already exited 1
# if they failed; this consolidation script only assembles the evidence).
exit 0
