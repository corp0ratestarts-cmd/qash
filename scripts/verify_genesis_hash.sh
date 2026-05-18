#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.."; pwd)"
manifest="${repo_root}/spec/genesis-artifacts.txt"
constants="${repo_root}/GENESIS_CONSTANTS.toml"

if [[ ! -f "${manifest}" ]]; then
  echo "ERROR: missing genesis artifact manifest: ${manifest}" >&2
  exit 1
fi

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

h = hashlib.sha3_256()
per_file: list[tuple[str, str, int]] = []
for rel in artifact_paths:
    path = repo_root / rel
    if not path.is_file():
        raise SystemExit(f"ERROR: listed genesis artifact is missing: {rel}")
    data = path.read_bytes()
    if rel == "GENESIS_CONSTANTS.toml":
        text = data.decode("utf-8")
        text, count = re.subn(
            r'(?m)^(genesis_hash\s*=\s*)"SHA3-256:[0-9a-fA-F<>A-Z_:-]+"',
            r'\1"SHA3-256:<SELF>"',
            text,
            count=1,
        )
        if count != 1:
            raise SystemExit("ERROR: could not canonicalize genesis_hash in GENESIS_CONSTANTS.toml")
        data = text.encode("utf-8")
    framed = rel.encode("utf-8") + b"\0" + str(len(data)).encode("ascii") + b"\0" + data + b"\0"
    h.update(framed)
    per_file.append((rel, hashlib.sha3_256(data).hexdigest(), len(data)))

computed = "SHA3-256:" + h.hexdigest()
constants_text = constants_path.read_text(encoding="utf-8")
match = re.search(r'(?m)^genesis_hash\s*=\s*"(SHA3-256:[0-9a-fA-F]+)"', constants_text)
if not match:
    raise SystemExit("ERROR: GENESIS_CONSTANTS.toml does not contain a concrete SHA3-256 genesis_hash")
recorded = match.group(1)

status_match = re.search(r'(?m)^genesis_status\s*=\s*"([^"]+)"', constants_text)
deploy_match = re.search(r'(?m)^deployment_authoritative\s*=\s*(true|false)', constants_text)
status = status_match.group(1) if status_match else "unspecified"
deploy = deploy_match.group(1) if deploy_match else "unspecified"

print(f"artifact_manifest={manifest_path.relative_to(repo_root)}")
print(f"artifact_count={len(per_file)}")
for rel, digest, size in per_file:
    print(f"artifact sha3-256={digest} bytes={size} path={rel}")
print(f"computed_genesis_hash={computed}")
print(f"recorded_genesis_hash={recorded}")
print(f"genesis_status={status}")
print(f"deployment_authoritative={deploy}")

if computed != recorded:
    raise SystemExit("ERROR: recorded genesis_hash does not match computed artifact-set hash")

pdf = repo_root / "spec/pdf/QASH_Spec_v1.0.pdf"
if not pdf.exists():
    if status != "provisional" or deploy != "false":
        raise SystemExit(
            "ERROR: normative PDF is absent; GENESIS_CONSTANTS.toml must mark "
            "genesis_status=\"provisional\" and deployment_authoritative=false"
        )
    print("notice=normative PDF absent; hash is provisional and not deployment-authoritative")
PY
