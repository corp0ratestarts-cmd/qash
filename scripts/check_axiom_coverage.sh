#!/usr/bin/env bash
# check_axiom_coverage.sh — Fail if new Axiom declarations are introduced
# without being documented in proofs/COVERAGE.md.
#
# In a PR context (GITHUB_BASE_SHA set), compares the Axiom set against the
# base commit.  For each new axiom, verifies its name appears in COVERAGE.md
# as a whole identifier (word-boundary match).
# On direct pushes (no base SHA), prints a summary and exits 0.
#
# Exit codes:
#   0 — No new axioms, or all new axioms are mentioned in COVERAGE.md.
#   1 — New axioms introduced without documentation in COVERAGE.md.
#   1 — Base SHA provided but could not be resolved (fail-closed).

set -euo pipefail

python3 - <<'PY'
import subprocess
import pathlib
import re
import sys
import os


def strip_coq_comments(text):
    """Remove (* ... *) comments (possibly nested) from Coq source."""
    result = []
    depth = 0
    i = 0
    while i < len(text):
        if text[i:i+2] == "(*":
            depth += 1
            i += 2
        elif text[i:i+2] == "*)":
            depth -= 1
            i += 2
        elif depth == 0:
            result.append(text[i])
            i += 1
        else:
            i += 1
    return "".join(result)


# Matches optional attribute(s) before the Axiom keyword, then captures the name.
# Handles: Axiom foo, #[local] Axiom foo, #[deprecated] #[local] Axiom foo
AXIOM_RE = re.compile(r'(?:#\[[^\]]*\]\s*)*Axiom\s+([A-Za-z_][A-Za-z0-9_\']*)')


def collect_axioms_from_text(text):
    """Return the set of Axiom names declared in Coq source text."""
    clean = strip_coq_comments(text)
    return {m.group(1) for m in AXIOM_RE.finditer(clean)}


def collect_axioms_from_worktree():
    """Return the set of Axiom names declared in active .v files."""
    result = set()
    for f in pathlib.Path("proofs").rglob("*.v"):
        if "_wip" in f.parts:
            continue
        result |= collect_axioms_from_text(f.read_text())
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
        result |= collect_axioms_from_text(content)
    return result


def axiom_in_coverage(name, cov_text):
    """Return True if `name` appears as a whole identifier in cov_text."""
    return bool(re.search(r'\b' + re.escape(name) + r'\b', cov_text))


base_sha = os.environ.get("GITHUB_BASE_SHA", "").strip()

if not base_sha:
    axioms = collect_axioms_from_worktree()
    print(f"Axiom summary ({len(axioms)} total): {sorted(axioms)}")
    sys.exit(0)

base_axioms = collect_axioms_from_ref(base_sha)
if base_axioms is None:
    print(f"ERROR: could not resolve base ref {base_sha!r}; failing closed.")
    sys.exit(1)

head_axioms = collect_axioms_from_worktree()
new_axioms = head_axioms - base_axioms

if not new_axioms:
    print(f"No new axioms introduced. ({len(head_axioms)} total) OK.")
    sys.exit(0)

# New axioms found — verify each name appears in COVERAGE.md as a whole identifier.
cov_text = pathlib.Path("proofs/COVERAGE.md").read_text()
missing = [a for a in sorted(new_axioms) if not axiom_in_coverage(a, cov_text)]

if missing:
    print("ERROR: New Axiom declarations not documented in proofs/COVERAGE.md:")
    for a in missing:
        print(f"  {a}")
    print()
    print("Add each axiom name to proofs/COVERAGE.md before merging.")
    sys.exit(1)

print(f"New axioms {sorted(new_axioms)} — all mentioned in COVERAGE.md. OK.")
PY
