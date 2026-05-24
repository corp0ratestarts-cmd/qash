#!/usr/bin/env bash
set -euo pipefail

bad='std::|HashMap|HashSet|SystemTime|Instant|thread_rng|OsRng|unsafe|unwrap\(|expect\(|panic!\(|unreachable!\('

if rg "$bad" crates/consensus/src crates/consensus/tests; then
  echo "Domain A forbidden pattern detected" >&2
  exit 1
fi

if rg "qash_pal|pal::|std::time|SystemTime|OsRng|thread_rng" crates/consensus/src; then
  echo "Domain A imports or references Domain B/nondeterministic surfaces" >&2
  exit 1
fi

echo "Domain A tripwire scan passed"
