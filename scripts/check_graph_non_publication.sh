#!/usr/bin/env bash
set -euo pipefail

# QASH is graph non-publishing. Public surfaces may expose roots and scalar
# commitments only, never graph structures, graph edges, adjacency data, raw tx
# topology, peer routing topology, or documentation that claims a public graph.

python3 - <<'PY'
from pathlib import Path
import re
import sys

checks = [
    (
        [Path('crates/consensus/src/public.rs'), Path('crates/pal/src/zero_wal.rs'), Path('crates/pal/src/receipt.rs')],
        [
            re.compile(r'\bPublicGraph\b'),
            re.compile(r'\bgraph_edges?\b', re.IGNORECASE),
            re.compile(r'\badjacenc(y|ies)\b', re.IGNORECASE),
            re.compile(r'\btopolog(y|ies)\b', re.IGNORECASE),
            re.compile(r'\broute_graph\b', re.IGNORECASE),
            re.compile(r'\bpeer_graph\b', re.IGNORECASE),
            re.compile(r'\btransaction_graph\b', re.IGNORECASE),
        ],
        'public/evidence code surface',
    ),
    (
        [Path('README.md'), Path('ROADMAP.md'), Path('PROJECT_STATUS.md')],
        [
            re.compile(r'\bpublic graph\b', re.IGNORECASE),
            re.compile(r'\bpublish(?:es|ed|ing)? the graph\b', re.IGNORECASE),
            re.compile(r'\bpublic transaction graph\b', re.IGNORECASE),
        ],
        'top-level documentation',
    ),
]

violations = []
for paths, patterns, label in checks:
    for path in paths:
        if not path.exists():
            continue
        for line_no, line in enumerate(path.read_text(encoding='utf-8').splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith('//') or stripped.startswith('//!') or stripped.startswith('///'):
                continue
            for pat in patterns:
                if pat.search(line):
                    violations.append(f'{label}: {path}:{line_no}: {line}')

if violations:
    print('graph non-publication violations:')
    print('\n'.join(violations))
    sys.exit(1)
PY

echo "graph non-publication guard passed"
