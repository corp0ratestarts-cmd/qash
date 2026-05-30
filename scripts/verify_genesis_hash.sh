#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="${repo_root}/spec/genesis-artifacts.txt"
constants="${repo_root}/GENESIS_CONSTANTS.toml"

if [[ ! -f "${manifest}" ]]; then
  echo "ERROR: missing genesis artifact manifest: ${manifest}" >&2
  exit 1
fi

# Step 1: Validate manifest (dedup, path safety, required entries) via Python.
python3 - "$repo_root" "$manifest" "$constants" <<'PY'
from __future__ import annotations

import hashlib
import pathlib
import re
import sys

repo_root = pathlib.Path(sys.argv[1])
manifest_path = pathlib.Path(sys.argv[2])
constants_path = pathlib.Path(sys.argv[3])

artifact_paths: list[str] = []
for raw_line in manifest_path.read_text(encoding="utf-8").splitlines():
    line = raw_line.strip()
    if not line or line.startswith("#"):
        continue
    if line.startswith("/") or ".." in pathlib.PurePosixPath(line).parts:
        raise SystemExit(f"ERROR: invalid artifact path in manifest: {line!r}")
    artifact_paths.append(line)

if len(artifact_paths) != len(set(artifact_paths)):
    seen: set[str] = set()
    duplicates = []
    for path in artifact_paths:
        if path in seen:
            duplicates.append(path)
        seen.add(path)
    raise SystemExit("ERROR: duplicate artifact path(s): " + ", ".join(duplicates))

if "GENESIS_CONSTANTS.toml" not in artifact_paths:
    raise SystemExit("ERROR: manifest must include GENESIS_CONSTANTS.toml")

per_file: list[tuple[str, str, int]] = []
for rel in artifact_paths:
    path = repo_root / rel
    if not path.is_file():
        raise SystemExit(f"ERROR: listed genesis artifact is missing: {rel}")
    data = path.read_bytes()
    per_file.append((rel, hashlib.sha3_256(data).hexdigest(), len(data)))

constants_text = constants_path.read_text(encoding="utf-8")
status_match = re.search(r'(?m)^genesis_status\s*=\s*"([^"]+)"', constants_text)
deploy_match = re.search(r'(?m)^deployment_authoritative\s*=\s*(true|false)', constants_text)
status = status_match.group(1) if status_match else "unspecified"
deploy = deploy_match.group(1) if deploy_match else "unspecified"

print(f"artifact_manifest={manifest_path.relative_to(repo_root)}")
print(f"artifact_count={len(per_file)}")
for rel, digest, size in per_file:
    print(f"artifact sha3-256={digest} bytes={size} path={rel}")
print(f"genesis_status={status}")
print(f"deployment_authoritative={deploy}")
PY

# Step 2: Compute the QASH-CASCADE-7 genesis hash via the Rust binary.
computed="$(cargo run -q --bin genesis-hash -- "${repo_root}")"

# Step 3: Extract the recorded genesis_hash from GENESIS_CONSTANTS.toml.
recorded="$(grep -E '^genesis_hash\s*=' "${constants}" \
  | sed -E 's/^genesis_hash\s*=\s*"([^"]+)".*/\1/')"

echo "computed_genesis_hash=${computed}"
echo "recorded_genesis_hash=${recorded}"

constants_text="$(cat "${constants}")"
status="$(echo "${constants_text}" | grep -E '^genesis_status' | sed -E 's/.*"([^"]+)".*/\1/' || echo unspecified)"
deploy="$(echo "${constants_text}" | grep -E '^deployment_authoritative' | awk '{print $3}' || echo unspecified)"

if [[ "${computed}" != "${recorded}" ]]; then
  if [[ "${status}" == "provisional" && "${deploy}" == "false" ]]; then
    echo "notice=recorded genesis_hash differs from computed artifact-set hash; allowed because genesis_status=provisional and deployment_authoritative=false"
  else
    echo "ERROR: recorded genesis_hash does not match computed artifact-set hash" >&2
    exit 1
  fi
fi

pdf="${repo_root}/spec/pdf/QASH_Spec_v1.0.pdf"
if [[ ! -f "${pdf}" ]]; then
  if [[ "${status}" != "provisional" || "${deploy}" != "false" ]]; then
    echo "ERROR: normative PDF is absent; GENESIS_CONSTANTS.toml must mark genesis_status=\"provisional\" and deployment_authoritative=false" >&2
    exit 1
  fi
  echo "notice=normative PDF absent; hash is provisional and not deployment-authoritative"
fi
