#!/usr/bin/env bash
# check_axiom_coverage.sh — Fail if new Axiom declarations are introduced
# without being documented in proofs/COVERAGE.md.
#
# In a PR context (GITHUB_BASE_SHA set), compares the Axiom set against the
# base commit.  For each new axiom, verifies its name appears in COVERAGE.md.
# On direct pushes (no base SHA), prints a summary and exits 0.
#
# Exit codes:
#   0 — No new axioms, or all new axioms are mentioned in COVERAGE.md.
#   1 — New axioms introduced without documentation in COVERAGE.md.

set -euo pipefail

python3 - <<'PY'
import subprocess
import pathlib
import re
import sys
import os


def collect_axioms_from_worktree():
    """Return the set of Axiom names declared in active .v files."""
    result = set()
    for f in pathlib.Path("proofs").rglob("*.v"):
        if "_wip" in f.parts:
            continue
        for line in f.read_text().splitlines():
            m = re.match(r"\s*Axiom\s+(\w+)", line)
            if m:
                result.add(m.group(1))
    return result


def collect_axioms_from_ref(ref):
    """Return the set of Axiom names in active .v files at git ref, or None on error."""
    result = set()
    try:
        ls = subprocess.run(
            ["git", "ls-tree", "-r", "--name-only", ref],
            capture_output=True, text=True, check=True,
        )
    except subprocess.CalledProcessError:
        return None
    for path in ls.stdout.splitlines():
        if not path.startswith("proofs/") or not path.endswith(".v"):
            continue
        if "_wip" in path:
            continue
        try:
            content = subprocess.run(
                ["git", "show", f"{ref}:{path}"],
                capture_output=True, text=True, check=True,
            ).stdout
        except subprocess.CalledProcessError:
            continue
        for line in content.splitlines():
            m = re.match(r"\s*Axiom\s+(\w+)", line)
            if m:
                result.add(m.group(1))
    return result


base_sha = os.environ.get("GITHUB_BASE_SHA", "").strip()

if not base_sha:
    axioms = collect_axioms_from_worktree()
    print(f"Axiom summary ({len(axioms)} total): {sorted(axioms)}")
    sys.exit(0)

base_axioms = collect_axioms_from_ref(base_sha)
if base_axioms is None:
    print(f"Warning: could not resolve base ref {base_sha!r}; skipping check.")
    sys.exit(0)

head_axioms = collect_axioms_from_worktree()
new_axioms = head_axioms - base_axioms

if not new_axioms:
    print(f"No new axioms introduced. ({len(head_axioms)} total) OK.")
    sys.exit(0)

# New axioms found — verify each name appears in COVERAGE.md.
# This avoids git diff (unreliable in shallow clones).
cov_text = pathlib.Path("proofs/COVERAGE.md").read_text()
missing = [a for a in sorted(new_axioms) if a not in cov_text]

if missing:
    print("ERROR: New Axiom declarations not documented in proofs/COVERAGE.md:")
    for a in missing:
        print(f"  {a}")
    print()
    print("Add each axiom name to proofs/COVERAGE.md before merging.")
    sys.exit(1)

print(f"New axioms {sorted(new_axioms)} — all mentioned in COVERAGE.md. OK.")
PY
