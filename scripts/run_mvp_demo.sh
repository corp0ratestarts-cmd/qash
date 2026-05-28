#!/usr/bin/env bash
# run_mvp_demo.sh — End-to-end MVP offline incident receipt commit demonstrator.
#
# Usage:
#   ./scripts/run_mvp_demo.sh [--clean]
#
# Exercises the full local demo flow:
#   init → issue-receipt × 2 → sync → replay → disclose → multi-import → dedup check
#
# Then verifies:
#   - public commitment export does NOT contain private incident body text
#   - selected disclosure verifies against the public commitment export
#   - selected disclosure contains the selected receipt body
#   - selected disclosure does NOT contain the other receipt body
#   - public evidence export excludes private disclosure/body material
#   - replay completes deterministically (root is stable on second run)
#
# Scope: Domain B demonstrator only. Not a production payment, settlement,
# credential, or genesis transaction system.
#
# Exit code: 0 on success, non-zero on any failure.

set -euo pipefail

ARTIFACT_DIR="artifacts/mvp-demo"
NODE_DIR="${ARTIFACT_DIR}/node-a"
NODE_B_DIR="${ARTIFACT_DIR}/node-b"
COMMITMENTS_FILE="${ARTIFACT_DIR}/public_commitments.bin"
NODE_B_EXPORT="${ARTIFACT_DIR}/node-b-commitments.bin"
DISCLOSURE_FILE="${ARTIFACT_DIR}/disclosure.bin"
EVIDENCE_DIR="${ARTIFACT_DIR}/public-evidence"

BODY_ONE="synthetic incident alpha"
BODY_TWO="synthetic incident beta"

if [[ "${1:-}" == "--clean" ]]; then
    rm -rf "${ARTIFACT_DIR}"
fi

mkdir -p "${ARTIFACT_DIR}"

echo "=== QASH MVP Demo ==="
echo "scope: Domain B offline incident receipt commit demonstrator"
echo "dir: ${NODE_DIR}"
echo ""

# ── Build ──────────────────────────────────────────────────────────────────
echo "[1/9] building qash-demo..."
cargo build --bin qash-demo --quiet

DEMO="cargo run --quiet --bin qash-demo --"

# ── Init ───────────────────────────────────────────────────────────────────
echo "[2/9] init workspace..."
$DEMO init --dir "${NODE_DIR}"

# ── Issue two receipts ─────────────────────────────────────────────────────
echo "[3/9] issuing first receipt..."
RECEIPT_ONE_OUTPUT=$($DEMO issue-receipt \
    --dir "${NODE_DIR}" \
    --epoch 1 \
    --body "${BODY_ONE}")
echo "${RECEIPT_ONE_OUTPUT}"

RECEIPT_ONE_ID=$(echo "${RECEIPT_ONE_OUTPUT}" | grep '^receipt_id:' | awk '{print $2}')
if [[ -z "${RECEIPT_ONE_ID}" ]]; then
    echo "ERROR: could not extract receipt_id from first issue-receipt output" >&2
    exit 1
fi

echo "[4/9] issuing second receipt..."
$DEMO issue-receipt \
    --dir "${NODE_DIR}" \
    --epoch 2 \
    --body "${BODY_TWO}"

# ── Sync ───────────────────────────────────────────────────────────────────
echo "[5/9] syncing commitment-only public export..."
$DEMO sync --dir "${NODE_DIR}" --out "${COMMITMENTS_FILE}"

# ── Replay ─────────────────────────────────────────────────────────────────
echo "[6/9] replaying..."
REPLAY_ONE=$($DEMO replay --dir "${NODE_DIR}")
echo "${REPLAY_ONE}"

ROOT_ONE=$(echo "${REPLAY_ONE}" | grep '^commitment_root:' | awk '{print $2}')

# Second replay must produce the same root (determinism check).
ROOT_TWO=$($DEMO replay --dir "${NODE_DIR}" | grep '^commitment_root:' | awk '{print $2}')
if [[ "${ROOT_ONE}" != "${ROOT_TWO}" ]]; then
    echo "ERROR: replay is not deterministic (root changed between runs)" >&2
    echo "  run 1: ${ROOT_ONE}" >&2
    echo "  run 2: ${ROOT_TWO}" >&2
    exit 1
fi
echo "determinism check: commitment_root is stable across two runs (${ROOT_ONE})"

# ── Disclose ───────────────────────────────────────────────────────────────
echo "[7/9] disclosing first receipt (${RECEIPT_ONE_ID})..."
$DEMO disclose \
    --dir "${NODE_DIR}" \
    --receipt-id "${RECEIPT_ONE_ID}" \
    --out "${DISCLOSURE_FILE}"

$DEMO verify-disclosure \
    --disclosure "${DISCLOSURE_FILE}" \
    --commitments "${COMMITMENTS_FILE}"

# ── Multi-operator import (v0.3) ───────────────────────────────────────────
echo "[8/9] multi-operator import: node-b issues a receipt and node-a imports it..."
$DEMO init --dir "${NODE_B_DIR}"
$DEMO issue-receipt \
    --dir "${NODE_B_DIR}" \
    --epoch 3 \
    --body "synthetic incident gamma"
$DEMO sync --dir "${NODE_B_DIR}" --out "${NODE_B_EXPORT}"

# node-a imports node-b's public commitments
$DEMO import-commitments \
    --dir "${NODE_DIR}" \
    --file "${NODE_B_EXPORT}" \
    --label "node-b"

$DEMO list-imports --dir "${NODE_DIR}"

# replay on node-a now includes node-b's records
REPLAY_MERGED=$($DEMO replay --dir "${NODE_DIR}")
echo "${REPLAY_MERGED}"
ROOT_MERGED=$(echo "${REPLAY_MERGED}" | grep '^commitment_root:' | awk '{print $2}')

# merged root must differ from single-node root (it now has an extra record)
if [[ "${ROOT_MERGED}" == "${ROOT_ONE}" ]]; then
    echo "ERROR: merged replay root is same as single-node root — import may not have been included" >&2
    exit 1
fi
echo "multi-import check: merged root differs from single-node root (expected)"
echo "  single-node root: ${ROOT_ONE}"
echo "  merged root:      ${ROOT_MERGED}"

# ── Idempotent re-import check ─────────────────────────────────────────────
echo "[9/9] idempotent re-import check: importing node-b again should produce 0 new records..."
REIMPORT_OUTPUT=$($DEMO import-commitments \
    --dir "${NODE_DIR}" \
    --file "${NODE_B_EXPORT}" \
    --label "node-b-dedup-check")
echo "${REIMPORT_OUTPUT}"
NEW_RECORDS=$(echo "${REIMPORT_OUTPUT}" | grep '^\s*new:' | awk '{print $2}')
if [[ "${NEW_RECORDS}" != "0" ]]; then
    echo "ERROR: expected 0 new records on re-import, got: ${NEW_RECORDS}" >&2
    exit 1
fi
echo "dedup check: 0 new records on re-import (all marked as duplicates)"

echo "[evidence] exporting public operator evidence bundle..."
$DEMO status --dir "${NODE_DIR}" --json
$DEMO export-evidence --dir "${NODE_DIR}" --out "${EVIDENCE_DIR}"
$DEMO verify-evidence --evidence "${EVIDENCE_DIR}"

# ── Assertions ─────────────────────────────────────────────────────────────
echo ""
echo "=== Verifying privacy boundaries ==="

fail=0

# Public commitments must not contain private incident body text.
if grep -qF "${BODY_ONE}" "${COMMITMENTS_FILE}" 2>/dev/null; then
    echo "FAIL: public commitments file contains body one text" >&2
    fail=1
else
    echo "PASS: public commitments do not contain '${BODY_ONE}'"
fi

if grep -qF "${BODY_TWO}" "${COMMITMENTS_FILE}" 2>/dev/null; then
    echo "FAIL: public commitments file contains body two text" >&2
    fail=1
else
    echo "PASS: public commitments do not contain '${BODY_TWO}'"
fi

if grep -R -qF "${BODY_ONE}" "${EVIDENCE_DIR}" 2>/dev/null; then
    echo "FAIL: public evidence bundle contains body one text" >&2
    fail=1
else
    echo "PASS: public evidence bundle does not contain '${BODY_ONE}'"
fi

if [[ -e "${EVIDENCE_DIR}/disclosure.bin" ]]; then
    echo "FAIL: public evidence bundle includes disclosure.bin" >&2
    fail=1
else
    echo "PASS: public evidence bundle excludes disclosure.bin"
fi

# Selected disclosure must contain the selected receipt body.
if grep -qF "${BODY_ONE}" "${DISCLOSURE_FILE}" 2>/dev/null; then
    echo "PASS: disclosure contains selected receipt body ('${BODY_ONE}')"
else
    echo "FAIL: disclosure does not contain selected receipt body" >&2
    fail=1
fi

# Selected disclosure must NOT contain the other receipt body.
if grep -qF "${BODY_TWO}" "${DISCLOSURE_FILE}" 2>/dev/null; then
    echo "FAIL: disclosure contains non-selected receipt body ('${BODY_TWO}')" >&2
    fail=1
else
    echo "PASS: disclosure does not contain non-selected receipt body ('${BODY_TWO}')"
fi

echo ""
if [[ "${fail}" -ne 0 ]]; then
    echo "RESULT: FAILED — privacy boundary violation detected" >&2
    exit 1
fi

echo "RESULT: PASSED"
echo ""
echo "Artifacts written to ${ARTIFACT_DIR}/"
echo "  ${COMMITMENTS_FILE}  (node-a commitment-only public export)"
echo "  ${NODE_B_EXPORT}  (node-b commitment-only public export)"
echo "  ${DISCLOSURE_FILE}  (selected receipt disclosure)"
echo "  ${EVIDENCE_DIR}/  (public evidence bundle)"
echo ""
echo "Claim boundary: this demonstrator is not a payment instrument,"
echo "settlement rail, credential system, or production deployment."
