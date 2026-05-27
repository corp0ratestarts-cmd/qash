#!/usr/bin/env bash
# audit_domain_boundary_full.sh — Phase 10 of the pre-genesis full-repo audit.
#
# Extends check_domain_a_tripwires.sh with a full scan of crates/consensus/src/
# for Domain B imports and platform/accelerator/hardware contamination patterns.
#
# Status: Blocking — exit 1 on any Domain A violation.
#
# Standard Domain B import contamination (blocking):
#   qash_pal::, qash_address::, use std::net, use std::fs, use std::env
#   std::time::, SystemTime, Instant, OsRng, getrandom, rand::
#   serde_json, log::, tracing::, tokio::, async fn, .await
#
# Platform/accelerator/hardware contamination (blocking):
#   itron::, freertos, zephyr, rtems, vxworks, qnx
#   cuda::, rocm::, musa::, opencl::, vulkan::, metal::, onedal::
#   tpm::, pkcs11::, javacard::, sgx::, trustzone::
set -euo pipefail

OUTPUT_DIR="artifacts/audit"
OUTPUT_FILE="$OUTPUT_DIR/domain_boundary_full.md"
mkdir -p "$OUTPUT_DIR"

COMMIT_SHA=$(git rev-parse HEAD)
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

DOMAIN_A_DIR="crates/consensus/src"

FAIL=0
VIOLATIONS=()

# ── Standard Domain B import patterns ─────────────────────────────────────────
DOMAIN_B_PATTERNS=(
  'qash_pal::'
  'qash_address::'
  'use std::net'
  'use std::fs'
  'use std::env'
  'std::time::'
  'SystemTime'
  'Instant'
  'OsRng'
  'getrandom'
  'rand::'
  'serde_json'
  'log::'
  'tracing::'
  'tokio::'
  'async[[:space:]]+fn'
  '\.await'
)

DOMAIN_B_LABELS=(
  'qash_pal:: (Domain B PAL import)'
  'qash_address:: (Domain B address import)'
  'use std::net (network I/O)'
  'use std::fs (filesystem I/O)'
  'use std::env (environment access)'
  'std::time:: (wall clock)'
  'SystemTime (wall clock)'
  'Instant (monotonic clock)'
  'OsRng (entropy)'
  'getrandom (entropy)'
  'rand:: (nondeterminism)'
  'serde_json (serialization coupling)'
  'log:: (logging/tracing)'
  'tracing:: (logging/tracing)'
  'tokio:: (async runtime)'
  'async fn (async function)'
  '.await (async suspension point)'
)

# ── Platform/accelerator/hardware contamination patterns ──────────────────────
PLATFORM_PATTERNS=(
  'itron::'
  '[Ff]reertos'
  '[Zz]ephyr'
  '[Rr]tems'
  '[Vv]xworks'
  '[Qq]nx'
  'cuda::'
  'rocm::'
  'musa::'
  'opencl::'
  'vulkan::'
  'metal::'
  'onedal::'
  'tpm::'
  'pkcs11::'
  'javacard::'
  'sgx::'
  'trustzone::'
)

PLATFORM_LABELS=(
  'itron:: (ITRON RTOS)'
  'freertos (FreeRTOS)'
  'zephyr (Zephyr RTOS)'
  'rtems (RTEMS)'
  'vxworks (VxWorks)'
  'qnx (QNX)'
  'cuda:: (NVIDIA CUDA)'
  'rocm:: (AMD ROCm)'
  'musa:: (Moore Threads MUSA)'
  'opencl:: (OpenCL)'
  'vulkan:: (Vulkan compute)'
  'metal:: (Apple Metal)'
  'onedal:: (Intel oneDAL)'
  'tpm:: (TPM)'
  'pkcs11:: (HSM/PKCS#11)'
  'javacard:: (JavaCard)'
  'sgx:: (SGX enclave)'
  'trustzone:: (TrustZone)'
)

# ── Scan function ─────────────────────────────────────────────────────────────
scan_pattern() {
  local file="$1"
  local pattern="$2"
  local label="$3"

  while IFS= read -r hit; do
    [ -z "$hit" ] && continue
    # Skip lines that are only comments
    local line_content
    line_content=$(echo "$hit" | sed 's/^[^:]*:[^:]*://')
    if echo "$line_content" | grep -qE '^\s*//'; then
      continue
    fi
    VIOLATIONS+=("[$label] $hit")
    FAIL=1
  done < <(grep -nP "$pattern" "$file" 2>/dev/null | sed "s|^|$file:|" || true)
}

# ── Run scan ──────────────────────────────────────────────────────────────────
if [ -d "$DOMAIN_A_DIR" ]; then
  echo "Scanning Domain A ($DOMAIN_A_DIR) for Domain B imports..."
  while IFS= read -r file; do
    for i in "${!DOMAIN_B_PATTERNS[@]}"; do
      scan_pattern "$file" "${DOMAIN_B_PATTERNS[$i]}" "${DOMAIN_B_LABELS[$i]}"
    done
    for i in "${!PLATFORM_PATTERNS[@]}"; do
      scan_pattern "$file" "${PLATFORM_PATTERNS[$i]}" "${PLATFORM_LABELS[$i]}"
    done
  done < <(find "$DOMAIN_A_DIR" -name '*.rs' 2>/dev/null)
else
  echo "Warning: Domain A directory '$DOMAIN_A_DIR' not found." >&2
fi

# ── Emit report ───────────────────────────────────────────────────────────────
{
  echo "# Domain A/B Full Boundary Scan"
  echo ""
  echo "**Commit:** \`$COMMIT_SHA\`  "
  echo "**Timestamp:** $TIMESTAMP  "
  echo "**Status:** $([ "$FAIL" -eq 0 ] && echo "✅ PASS — Domain A boundary is clean" || echo "❌ FAIL — ${#VIOLATIONS[@]} violation(s)")"
  echo ""
  echo "## Scope"
  echo ""
  echo "Extends \`check_domain_a_tripwires.sh\` with a full scan of \`$DOMAIN_A_DIR\`"
  echo "for Domain B imports and platform/accelerator/hardware contamination."
  echo ""
  echo "## Standard Domain B import patterns (blocking)"
  echo ""
  echo "| Pattern | Label |"
  echo "|---------|-------|"
  for i in "${!DOMAIN_B_PATTERNS[@]}"; do
    echo "| \`${DOMAIN_B_PATTERNS[$i]}\` | ${DOMAIN_B_LABELS[$i]} |"
  done
  echo ""
  echo "## Platform/accelerator/hardware patterns (blocking)"
  echo ""
  echo "| Pattern | Label |"
  echo "|---------|-------|"
  for i in "${!PLATFORM_PATTERNS[@]}"; do
    echo "| \`${PLATFORM_PATTERNS[$i]}\` | ${PLATFORM_LABELS[$i]} |"
  done
  echo ""
  echo "## Results"
  echo ""
  if [ "${#VIOLATIONS[@]}" -eq 0 ]; then
    echo "✅ **No violations found.** Domain A boundary is clean."
    echo ""
    echo "- No Domain B imports in \`$DOMAIN_A_DIR\`"
    echo "- No RTOS, accelerator, or hardware contamination"
  else
    echo "❌ **${#VIOLATIONS[@]} violation(s) — each is a blocking failure:**"
    echo ""
    for v in "${VIOLATIONS[@]}"; do
      echo "- \`$v\`"
    done
    echo ""
    echo "**Resolution:** Remove all Domain B imports and platform-specific references"
    echo "from \`$DOMAIN_A_DIR\`. Domain A must be a pure no_std deterministic replay"
    echo "kernel with no runtime, OS, hardware, or accelerator coupling."
  fi
  echo ""
  echo "## Policy reminder"
  echo ""
  echo "> Domain A is the deterministic \`no_std\` replay kernel. RTOS, accelerator,"
  echo "> and hardware profiles belong in Domain B PAL adapters. They must not alter"
  echo "> Domain A state-root semantics, introduce nondeterminism, or compromise"
  echo "> replay equivalence across authorised ISAs."
  echo ""
  echo "## Verdict"
  echo ""
  if [ "$FAIL" -eq 0 ]; then
    echo "**PASS** — Domain A/B boundary is intact."
  else
    echo "**FAIL** — ${#VIOLATIONS[@]} violation(s) must be removed before genesis-lock."
  fi
} > "$OUTPUT_FILE"

echo ""
echo "Domain boundary full scan complete."
echo "  Violations: ${#VIOLATIONS[@]}"
echo "  Report: $OUTPUT_FILE"

if [ "$FAIL" -ne 0 ]; then
  echo "  BLOCKING: ${#VIOLATIONS[@]} Domain A violation(s)." >&2
  exit 1
fi
echo "  PASS"
