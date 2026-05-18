#!/usr/bin/env bash
set -euo pipefail

PINNED_VERSION=$(
  python3 - <<'PY'
from pathlib import Path
import re
text = Path("rust-toolchain.toml").read_text(encoding="utf-8")
match = re.search(r'^\s*channel\s*=\s*"([^"]+)"\s*$', text, re.MULTILINE)
if not match:
    raise SystemExit("rust-toolchain.toml does not define [toolchain].channel")
print(match.group(1))
PY
)

VERSION_OUTPUT=$(rustc --version --verbose)
printf '%s\n' "$VERSION_OUTPUT"

ACTUAL_VERSION=$(printf '%s\n' "$VERSION_OUTPUT" | awk -F': ' '/^release: / {print $2}')
if [ "$ACTUAL_VERSION" != "$PINNED_VERSION" ]; then
  echo "ERROR: rustc release ${ACTUAL_VERSION:-<unknown>} does not match pinned Rust ${PINNED_VERSION}" >&2
  exit 1
fi

echo "OK: rustc release matches pinned Rust ${PINNED_VERSION}"
