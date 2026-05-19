#!/usr/bin/env python3
"""Compile proof obligations from git diff and coverage artifacts.

Skeleton utility for CI policy checks around Domain A proof obligations.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
ALLOWED_STATUS = {"PROVED", "CI-VERIFIED", "AXIOM"}
DOMAIN_A_REGIONS = {
    "crates/consensus/src/transition.rs": ["TH5", "TH6", "RT1", "RT2", "RT3", "RT4"],
    "crates/consensus/src/lyapunov.rs": ["TH3A", "TH3B", "TH3C", "TH_GC"],
    "crates/consensus/src/causal_order.rs": ["CAUSAL_ORDER"],
}


@dataclass
class ObligationResult:
    changed_paths: list[str]
    required_tags: dict[str, list[str]]


def git_changed_paths(base: str | None = None) -> list[str]:
    if base:
        cmd = ["git", "diff", "--name-only", f"{base}...HEAD"]
    else:
        cmd = ["git", "diff", "--name-only", "HEAD"]
    out = subprocess.check_output(cmd, cwd=REPO_ROOT, text=True)
    return [line.strip() for line in out.splitlines() if line.strip()]


def compile_obligations(paths: list[str]) -> ObligationResult:
    required: dict[str, list[str]] = {}
    for path in paths:
        tags = DOMAIN_A_REGIONS.get(path)
        if tags:
            required[path] = tags
    return ObligationResult(changed_paths=paths, required_tags=required)


def markdown_rows(markdown_text: str) -> list[str]:
    return [ln for ln in markdown_text.splitlines() if ln.strip().startswith("|")]


def enforce_coverage_update(paths: list[str]) -> None:
    touched_domain_a = any(path in DOMAIN_A_REGIONS for path in paths)
    if touched_domain_a and "proofs/COVERAGE.md" not in paths:
        raise SystemExit(
            "Domain A semantics changed without proofs/COVERAGE.md update. "
            "Add a row update that tracks the proof obligation impact."
        )


def ensure_no_untracked_domain_a_obligations(result: ObligationResult, coverage_text: str) -> None:
    if not result.required_tags:
        return
    for tags in result.required_tags.values():
        for tag in tags:
            if tag == "CAUSAL_ORDER":
                # TODO: introduce dedicated causal ordering theorem family in COVERAGE.md.
                continue
            if tag not in coverage_text:
                raise SystemExit(
                    f"Domain A semantics touched but obligation tag {tag!r} is not tracked in proofs/COVERAGE.md"
                )


def generate_coverage_json(md_path: Path, json_path: Path) -> None:
    text = md_path.read_text(encoding="utf-8")
    rows = markdown_rows(text)
    status_counts = {status: len(re.findall(rf"\*\*{re.escape(status)}\*\*", text)) for status in sorted(ALLOWED_STATUS)}
    payload = {
        "source": str(md_path.as_posix()),
        "status_allowlist": sorted(ALLOWED_STATUS),
        "status_counts": status_counts,
        "table_rows": rows,
    }
    json_path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default=os.environ.get("GITHUB_BASE_SHA"))
    parser.add_argument("--emit-json", action="store_true")
    args = parser.parse_args()

    changed = git_changed_paths(args.base)
    result = compile_obligations(changed)

    coverage_md = REPO_ROOT / "proofs" / "COVERAGE.md"
    coverage_text = coverage_md.read_text(encoding="utf-8")
    enforce_coverage_update(changed)
    ensure_no_untracked_domain_a_obligations(result, coverage_text)

    if args.emit_json:
        generate_coverage_json(coverage_md, REPO_ROOT / "proofs" / "coverage.json")

    print(json.dumps({"changed_paths": result.changed_paths, "required_tags": result.required_tags}, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
