#!/usr/bin/env bash
set -euo pipefail

python3 - <<'PY'
from pathlib import Path
import re
import tomllib

readme = Path('README.md')
constants = Path('GENESIS_CONSTANTS.toml')
if not readme.exists() or not constants.exists():
    raise SystemExit('ERROR: run from repo root with README.md and GENESIS_CONSTANTS.toml present')

data = tomllib.loads(constants.read_text(encoding='utf-8'))
text = readme.read_text(encoding='utf-8')

checks = [
    ("weight_divergence_D", data["lyapunov"]["weight_divergence_D"], r"weight_divergence_D[^\n]*?([0-9][0-9_,]*)"),
    ("weight_conflict_C", data["lyapunov"]["weight_conflict_C"], r"weight_conflict_C[^\n]*?([0-9][0-9_,]*)"),
    ("weight_slash_Sigma", data["lyapunov"]["weight_slash_Sigma"], r"weight_slash_Sigma[^\n]*?([0-9][0-9_,]*)"),
    ("duration_ms", data["epoch"]["timing"]["duration_ms"], r"duration_ms[^\n]*?([0-9][0-9_,]*)"),
]

errors = 0
for name, expected, pat in checks:
    matches = list(re.finditer(pat, text, flags=re.IGNORECASE))
    if len(matches) == 0:
        print(f"SKIP: {name} not found in README")
        continue
    if len(matches) > 1:
        print(f"FAIL: {name}: expected one contextual match in README, found {len(matches)}")
        errors += 1
        continue
    raw = matches[0].group(1)
    found = int(raw.replace('_','').replace(',',''))
    if found != int(expected):
        print(f"FAIL: {name}: README={found} TOML={expected}")
        errors += 1
    else:
        print(f"OK: {name}={expected}")

if errors:
    raise SystemExit(f"verify_readme_constants: {errors} check(s) failed")
print('verify_readme_constants: all checks passed')
PY
