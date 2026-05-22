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


GATE_DOC = pathlib.Path("docs/release/genesis_lock_gates.md")
EXPECTED_BLOCKERS = {
    "traceability artifact reconciliation": "Traceability artifact reconciliation",
    "normative pdf finalization": "Normative PDF finalization",
    "cross-isa replay evidence review": "Cross-ISA replay evidence review",
    "pal/network readiness decision": "PAL/network readiness decision",
}


def parse_gate_rows(lines):
    rows = []
    for line in lines:
        stripped = line.strip()
        if not stripped.startswith("|"):
            continue
        if stripped.startswith("| Blocker |") or stripped.startswith("|---"):
            continue
        cols = [c.strip() for c in stripped.strip("|").split("|")]
        if len(cols) < 4:
            continue
        rows.append(cols[:4])
    return rows


def normalize_blocker(value):
    value = value.strip().lower()
    value = re.sub(r"\s+", " ", value)
    return value


def gate_doc_failures():
    failures_local = []

    if not GATE_DOC.is_file():
        return [f"{GATE_DOC}: missing genesis lock gate source-of-truth document"]

    gate_text = GATE_DOC.read_text(encoding="utf-8")
    gate_lines = gate_text.splitlines()

    rows = parse_gate_rows(gate_lines)
    if not rows:
        failures_local.append(f"{GATE_DOC}: missing blocker table rows")
        return failures_local

    seen = {}
    for idx, row in enumerate(rows, start=1):
        blocker, owner, criterion, evidence = row
        label = f"{GATE_DOC}: table row {idx}"

        if not blocker or not owner or not criterion or not evidence:
            failures_local.append(f"{label}: blocker, owner, criterion, and evidence fields must all be non-empty")
            continue

        key = normalize_blocker(blocker)
        seen[key] = label

        if "TODO" in owner.upper() or "TODO" in criterion.upper() or "TODO" in evidence.upper():
            failures_local.append(f"{label}: gate fields must be current (no TODO placeholders)")

    for expected_key, expected_label in EXPECTED_BLOCKERS.items():
        if expected_key not in seen:
            failures_local.append(f"{GATE_DOC}: missing blocker row '{expected_label}'")

    status_text = pathlib.Path("proofs/STATUS.md").read_text(encoding="utf-8")
    status_text_lower = status_text.lower()
    missing_mentions = []
    for blocker_key in EXPECTED_BLOCKERS:
        pattern_parts = [re.escape(part) for part in blocker_key.split()]
        pattern_text = r"\b" + r"\s+".join(pattern_parts) + r"\b"
        if blocker_key == "pal/network readiness decision":
            pattern_text = pattern_text[:-2] + r"s?\b"
        pattern = re.compile(pattern_text)
        if not pattern.search(status_text_lower):
            missing_mentions.append(blocker_key)
    if missing_mentions:
        missing = ", ".join(EXPECTED_BLOCKERS[b] for b in missing_mentions)
        failures_local.append(f"proofs/STATUS.md: expected genesis blockers not found: {missing}")

    if "spec/pdf/QASH_Spec_v1.0.pdf" not in gate_text:
        failures_local.append(f"{GATE_DOC}: normative PDF gate must reference spec/pdf/QASH_Spec_v1.0.pdf")

    return failures_local


tracked = subprocess.run(
    ["git", "ls-files"],
    check=True,
    capture_output=True,
    text=True,
).stdout.splitlines()

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

failures.extend(gate_doc_failures())

if failures:
    print("Document hygiene check failed:")
    for failure in failures:
        print(f"  - {failure}")
    sys.exit(1)

print("Document hygiene check passed.")
PY
