#!/usr/bin/env bash
set -euo pipefail

python - <<'PY'
from pathlib import Path
import re
import sys

spec_dir = Path('docs/spec')
files = sorted(spec_dir.glob('*.md'))

section_re = re.compile(r'^##\s+.*\bTX-(\d+)\b', re.IGNORECASE)
required_patterns = {
    'epoch_unlinkable_identity_scheme': re.compile(r'\bepoch_unlinkable_identity_scheme\b'),
    'receipt_privacy.body_encryption': re.compile(r'\breceipt_privacy\s*:\s*(?:\n|.)*?\bbody_encryption\b', re.IGNORECASE),
    'receipt_privacy.disclosure_domain': re.compile(r'\breceipt_privacy\s*:\s*(?:\n|.)*?\bdisclosure_domain\b', re.IGNORECASE),
    'receipt_privacy.plaintext_at_halt = false': re.compile(r'\breceipt_privacy\s*:\s*(?:\n|.)*?\bplaintext_at_halt\s*:\s*false\b', re.IGNORECASE),
}

failures = []
checked_sections = 0

for file in files:
    text = file.read_text(encoding='utf-8')
    lines = text.splitlines()

    starts = []
    for i, line in enumerate(lines):
        m = section_re.match(line)
        if m:
            tx_id = int(m.group(1))
            if tx_id >= 2:
                starts.append((i, tx_id, line.strip()))

    for idx, tx_id, heading in starts:
        checked_sections += 1
        end = len(lines)
        for j in range(idx + 1, len(lines)):
            if lines[j].startswith('## '):
                end = j
                break

        section = '\n'.join(lines[idx:end])
        missing = [name for name, pat in required_patterns.items() if not pat.search(section)]
        if missing:
            failures.append((file.as_posix(), tx_id, heading, missing))

if failures:
    print('Privacy admission spec-lint failed for TX-2+ sections:')
    for path, tx_id, heading, missing in failures:
        print(f'- {path} :: {heading}')
        for item in missing:
            print(f'  - missing: {item}')
    sys.exit(1)

print(f'privacy admission spec-lint passed ({checked_sections} TX-2+ sections checked)')
PY
