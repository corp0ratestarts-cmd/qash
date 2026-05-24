#!/usr/bin/env bash
# Reject raw transcript dumps and ad hoc root-level spec files.

set -euo pipefail

python3 - <<'PY'
import pathlib
import re
import subprocess
import sys

ROOT_ALLOW = {
    ".gitignore",
    "AGENTS.md",
    "ARCHITECTURE.md",
    "build.rs",
    "Cargo.lock",
    "Cargo.toml",
    "CONTRIBUTING.md",
    "GENESIS_CONSTANTS.toml",
    "LICENSE",
    "PROJECT_STATUS.md",
    "README.md",
    "ROADMAP.md",
    "apply_patches.sh",
    "clippy.toml",
    "deny.toml",
    "design_decisions.md",
    "fix_workflows.sh",
    "rust-toolchain.toml",
}

ROOT_ALLOWED_SUFFIXES = {
    ".gitignore",
    ".md",
    ".toml",
    ".lock",
}

CANONICAL_PREFIXES = (
    ".github/",
    "artifacts/",
    "crates/",
    "docker/",
    "docs/",
    "fuzz/",
    "model/",
    "proofs/",
    "scripts/",
    "spec/",
    "src/",
    "tests/",
)

TRANSCRIPT_MARKER = re.compile(
    r"(?m)^(###\s+(USER|ASSISTANT|SYSTEM|DEVELOPER)\b|chat-[^\n]*Protocol Design Review)"
)

failures = []

CANONICAL_REPLAY_ENTRYPOINT = "scripts/replay_test.sh"

tracked = subprocess.run(
    ["git", "ls-files"],
    check=True,
    capture_output=True,
    text=True,
).stdout.splitlines()

changed = subprocess.run(
    ["git", "diff", "--name-status", "--cached"],
    check=True,
    capture_output=True,
    text=True,
).stdout.splitlines()

for row in changed:
    if not row:
        continue
    cols = row.split("\t")
    status = cols[0]
    if status != "A" or len(cols) < 2:
        continue
    rel = cols[1]
    if not rel.startswith("scripts/") or not rel.endswith(".sh"):
        continue
    if rel == CANONICAL_REPLAY_ENTRYPOINT:
        continue
    name = pathlib.PurePosixPath(rel).name
    if "replay" in name:
        failures.append(
            f"{rel}: new replay wrapper script detected; consolidate under {CANONICAL_REPLAY_ENTRYPOINT} or document deprecation/removal of superseded wrappers"
        )

for rel in tracked:
    path = pathlib.Path(rel)
    if not path.is_file():
        continue
    parts = pathlib.PurePosixPath(rel).parts
    is_root = len(parts) == 1

    if is_root:
        name = parts[0]
        if name not in ROOT_ALLOW:
            suffix = pathlib.PurePosixPath(name).suffix
            if suffix not in ROOT_ALLOWED_SUFFIXES or name.isdigit():
                failures.append(f"{rel}: root-level ad hoc file; move curated content under docs/spec, docs/adr, docs/traceability, or another canonical tree")

    if rel.startswith(CANONICAL_PREFIXES):
        scan_text = True
    elif is_root and parts[0] in ROOT_ALLOW:
        scan_text = True
    else:
        scan_text = False

    if not scan_text:
        continue

    try:
        text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        continue

    if TRANSCRIPT_MARKER.search(text):
        failures.append(f"{rel}: appears to contain a raw chat transcript or prompt dump")

if failures:
    print("Document hygiene check failed:")
    for failure in failures:
        print(f"  - {failure}")
    sys.exit(1)

print("Document hygiene check passed.")
PY
