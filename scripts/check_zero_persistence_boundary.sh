#!/usr/bin/env bash
set -euo pipefail

# Guard the production zero-persistence PAL path. The hosted replay scaffold may
# still contain raw fixture handling, but production admission, receipt privacy,
# WAL modules, commitment inbox/transport/recovery, graph non-publication, and
# public transcript surfaces must remain commitment-only.

cargo check -p qash-pal --features zero-persistence --no-default-features
cargo test -p qash-pal --features zero-persistence --test zero_persistence
cargo test -p qash-pal --features zero-persistence --test zero_persistence_profile
cargo test -p qash-pal --features zero-persistence --test ephemeral_traits
cargo test -p qash-pal --features zero-persistence --test receipt_privacy
cargo test -p qash-pal --features zero-persistence --test commitment_transport_recovery
cargo test -p qash-pal --features zero-persistence --test commitment_inbox
cargo test -p qash-consensus --test public_transcript_privacy
bash scripts/check_graph_non_publication.sh

python3 - <<'PY'
from pathlib import Path
import re
import sys

checks = [
    (
        [
            Path('crates/pal/src/admission.rs'),
            Path('crates/pal/src/commitment_inbox.rs'),
            Path('crates/pal/src/commitment_transport.rs'),
            Path('crates/pal/src/receipt.rs'),
            Path('crates/pal/src/recovery_wal.rs'),
            Path('crates/pal/src/zero_wal.rs'),
        ],
        [
            re.compile(r'\braw_txs\b'),
            re.compile(r'\braw_tx\b'),
            re.compile(r'\bpeer_ip\b'),
            re.compile(r'\bsocket_addr\b'),
            re.compile(r'\bString\b'),
            re.compile(r'\.to_vec\s*\('),
            re.compile(r'\.clone\s*\('),
        ],
        'zero-persistence boundary',
    ),
    (
        [Path('crates/consensus/src/public.rs')],
        [
            re.compile(r'\braw\b', re.IGNORECASE),
            re.compile(r'\bpayload\b', re.IGNORECASE),
            re.compile(r'\bgraph\b', re.IGNORECASE),
            re.compile(r'\bedge\b', re.IGNORECASE),
            re.compile(r'\bpeer\b', re.IGNORECASE),
            re.compile(r'\bsocket\b', re.IGNORECASE),
            re.compile(r'\bip\b', re.IGNORECASE),
            re.compile(r'\breceipt_body\b', re.IGNORECASE),
            re.compile(r'\bhardware\b', re.IGNORECASE),
            re.compile(r'\bserial\b', re.IGNORECASE),
            re.compile(r'\baaguid\b', re.IGNORECASE),
            re.compile(r'\boperator\b', re.IGNORECASE),
        ],
        'public transcript privacy',
    ),
]

violations = []
for paths, patterns, label in checks:
    for path in paths:
        for line_no, line in enumerate(path.read_text(encoding='utf-8').splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith('//') or stripped.startswith('//!') or stripped.startswith('///'):
                continue
            for pat in patterns:
                if pat.search(line):
                    violations.append(f'{label}: {path}:{line_no}: {line}')

if violations:
    print('privacy boundary violations:')
    print('\n'.join(violations))
    sys.exit(1)
PY

if rg -n '^\s*(Raw|Payload|Tx)[A-Za-z0-9_]*\b' crates/pal/src/zero_wal.rs; then
  echo "zero-persistence WAL must not expose raw/payload/tx variants" >&2
  exit 1
fi

echo "zero-persistence boundary gate passed"
