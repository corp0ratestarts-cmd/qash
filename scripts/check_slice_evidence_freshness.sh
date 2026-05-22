#!/usr/bin/env bash
set -euo pipefail

if [[ "${GITHUB_EVENT_NAME:-}" != "pull_request" ]]; then
  echo "Skipping slice evidence freshness check outside pull_request events."
  exit 0
fi

base_sha="${GITHUB_BASE_SHA:-}"
if [[ -z "$base_sha" ]]; then
  echo "GITHUB_BASE_SHA is required for pull_request checks." >&2
  exit 1
fi

git fetch --no-tags --depth=1 origin "$base_sha" >/dev/null 2>&1 || true
changed_files="$(git diff --name-only "$base_sha"...HEAD)"

critical_pattern='^(docs/spec/12_sharded_protocol\.md|crates/consensus/src/sharding\.rs|crates/consensus/tests/v1_2_sharded_replay\.rs|tests/vectors/vectors\.v1\.2\.json|proofs/sharding/efb_determinism\.v|crates/pal/src/lib\.rs|crates/pal/tests/hosted_replay\.rs|crates/pal/tests/whole_protocol\.rs|crates/pal/tests/boundary_violations\.rs|crates/pal/tests/smartcard\.rs|proofs/Makefile|proofs/STATUS\.md|proofs/_CoqProject|proofs/composition/th3_system_closure\.v|proofs/model/Model\.v|proofs/model/transition_observations\.json|crates/consensus/tests/coq_refinement_vectors\.rs|\.github/PULL_REQUEST_TEMPLATE\.md|\.github/workflows/ci\.yml|scripts/check_document_hygiene\.sh|scripts/capture_pre_genesis_evidence\.sh|docs/adr/ADR-006-runtime-optimization-track\.md|docs/release/pre_genesis_evidence_snapshot\.md|crates/consensus/tests/phase2r_preconditions\.rs|crates/consensus/benches/epoch_transition\.rs)$'

if ! printf '%s\n' "$changed_files" | rg -q "$critical_pattern"; then
  echo "No slice-critical file changes detected."
  exit 0
fi

manifest_path="$(printf '%s\n' "$changed_files" | rg '^artifacts/evidence/.+/manifest\.txt$' | tail -n 1 || true)"
if [[ -z "$manifest_path" ]]; then
  echo "Slice-critical changes require committing artifacts/evidence/*/manifest.txt." >&2
  exit 1
fi

if ! rg -q '^Captured \(UTC\): [0-9]{8}T[0-9]{6}Z$' "$manifest_path"; then
  echo "Manifest missing Captured (UTC) field: $manifest_path" >&2
  exit 1
fi
if ! rg -q '^Evidence freshness timestamp \(UTC\): [0-9]{8}T[0-9]{6}Z$' "$manifest_path"; then
  echo "Manifest missing evidence freshness timestamp: $manifest_path" >&2
  exit 1
fi
if ! rg -q '^Commit: [0-9a-f]{40}$' "$manifest_path"; then
  echo "Manifest missing full commit SHA: $manifest_path" >&2
  exit 1
fi
if ! rg -q '^Commit short: [0-9a-f]{12}$' "$manifest_path"; then
  echo "Manifest missing short commit SHA: $manifest_path" >&2
  exit 1
fi

python3 - "$manifest_path" <<'PY'
import re, sys
from pathlib import Path
text = Path(sys.argv[1]).read_text(encoding='utf-8')
if '## Slice Command Statuses' not in text:
    raise SystemExit('Manifest missing ## Slice Command Statuses section')
rows = [ln for ln in text.splitlines() if ln.startswith('| Slice ')]
if not rows:
    raise SystemExit('Manifest has no slice command status rows')
for row in rows:
    cols = [c.strip() for c in row.strip('|').split('|')]
    if len(cols) < 4:
        raise SystemExit(f'Malformed row: {row}')
    command, status = cols[1], cols[2]
    if status != 'PASS':
        raise SystemExit(f'Non-PASS slice command status for {command}: {status}')
PY

pr_body_file="${GITHUB_EVENT_PATH:-}"
if [[ ! -f "$pr_body_file" ]]; then
  echo "GITHUB_EVENT_PATH is missing; cannot validate PR body link." >&2
  exit 1
fi

if ! python3 - "$pr_body_file" "$manifest_path" <<'PY'
import json, sys
payload = json.load(open(sys.argv[1], encoding='utf-8'))
body = (payload.get('pull_request') or {}).get('body') or ''
manifest = sys.argv[2]
if manifest not in body:
    raise SystemExit(1)
PY
then
  echo "PR description must include the exact manifest path: $manifest_path" >&2
  exit 1
fi

echo "Slice evidence freshness check passed with $manifest_path"
