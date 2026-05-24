#!/usr/bin/env bash
set -euo pipefail

# Guard the production zero-persistence PAL path. The hosted replay scaffold may
# still contain raw fixture handling, but production admission, receipt privacy,
# and WAL modules must remain commitment-only.

cargo check -p qash-pal --features zero-persistence --no-default-features
cargo test -p qash-pal --features zero-persistence --test zero_persistence
cargo test -p qash-pal --features zero-persistence --test zero_persistence_profile
cargo test -p qash-pal --features zero-persistence --test ephemeral_traits
cargo test -p qash-pal --features zero-persistence --test receipt_privacy

python3 - <<'PY'
from pathlib import Path
import re
import sys

paths = [
    Path('crates/pal/src/admission.rs'),
    Path('crates/pal/src/receipt.rs'),
    Path('crates/pal/src/zero_wal.rs'),
]
patterns = [
    re.compile(r'\braw_txs\b'),
    re.compile(r'\braw_tx\b'),
    re.compile(r'\bpeer_ip\b'),
    re.compile(r'\bsocket_addr\b'),
    re.compile(r'\bString\b'),
    re.compile(r'\.to_vec\s*\('),
    re.compile(r'\.clone\s*\('),
]
violations = []
for path in paths:
    for line_no, line in enumerate(path.read_text(encoding='utf-8').splitlines(), 1):
        stripped = line.strip()
        if stripped.startswith('//') or stripped.startswith('//!') or stripped.startswith('///'):
            continue
        for pat in patterns:
            if pat.search(line):
                violations.append(f'{path}:{line_no}: {line}')

if violations:
    print('zero-persistence boundary violations:')
    print('\n'.join(violations))
    sys.exit(1)
PY

if rg -n '^\s*(Raw|Payload|Tx)[A-Za-z0-9_]*\b' crates/pal/src/zero_wal.rs; then
  echo "zero-persistence WAL must not expose raw/payload/tx variants" >&2
  exit 1
fi

echo "zero-persistence boundary gate passed"
