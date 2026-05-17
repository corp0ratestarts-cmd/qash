#!/usr/bin/env bash
# check_axiom_coverage.sh — Fail if new Axiom declarations are introduced
# without a corresponding update to proofs/COVERAGE.md.
#
# In a PR context (GITHUB_BASE_SHA set), compares the Axiom set in the PR
# against the base commit.  On direct pushes to main (no base SHA), prints a
# summary of all axioms and exits 0.
#
# Exit codes:
#   0 — No new axioms, or new axioms are documented in COVERAGE.md.
#   1 — New axioms introduced without updating COVERAGE.md.

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
    """Return the set of Axiom names declared in active .v files at git ref."""
    result = set()
    try:
        ls = subprocess.run(
            ["git", "ls-tree", "-r", "--name-only", ref],
            capture_output=True, text=True, check=True,
        )
    except subprocess.CalledProcessError:
        return None  # ref not found
    for path in ls.stdout.splitlines():
        if not path.startswith("proofs/") or not path.endswith(".v"):
            continue
        if "_wip" in path:
            continue
        content = subprocess.run(
            ["git", "show", f"{ref}:{path}"],
            capture_output=True, text=True,
        ).stdout
        for line in content.splitlines():
            m = re.match(r"\s*Axiom\s+(\w+)", line)
            if m:
                result.add(m.group(1))
    return result


base_sha = os.environ.get("GITHUB_BASE_SHA", "").strip()

if not base_sha:
    # Not a PR — summarise all axioms and exit cleanly.
    axioms = collect_axioms_from_worktree()
    print(f"Axiom summary ({len(axioms)} total): {sorted(axioms)}")
    sys.exit(0)

base_axioms = collect_axioms_from_ref(base_sha)
if base_axioms is None:
    print(f"Warning: could not resolve base SHA {base_sha!r}; skipping check.")
    sys.exit(0)

head_axioms = collect_axioms_from_worktree()
new_axioms = head_axioms - base_axioms

if not new_axioms:
    print(f"No new axioms introduced. ({len(head_axioms)} total) OK.")
    sys.exit(0)

# New axioms found — require COVERAGE.md to have been updated in this PR.
cov_diff = subprocess.run(
    ["git", "diff", "--name-only", base_sha, "HEAD", "--", "proofs/COVERAGE.md"],
    capture_output=True, text=True,
).stdout.strip()

if not cov_diff:
    print("ERROR: New Axiom declarations introduced without updating proofs/COVERAGE.md.")
    print("New axioms:")
    for a in sorted(new_axioms):
        print(f"  {a}")
    print()
    print("Document each new axiom in proofs/COVERAGE.md before merging.")
    sys.exit(1)

print(f"New axioms {sorted(new_axioms)} — COVERAGE.md updated. OK.")
PY
