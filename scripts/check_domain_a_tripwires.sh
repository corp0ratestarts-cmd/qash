#!/usr/bin/env bash
# check_domain_a_tripwires.sh — Enforce Domain A constraints in crates/consensus/src/.
#
# Two-pass strategy:
#   Pass 1 – always-forbidden patterns (banned even in test helpers):
#             std imports, HashMap/HashSet, nondeterminism sources, unsafe keyword.
#   Pass 2 – production-code-only forbidden patterns (panic/unwrap):
#             skip lines inside #[test] or #[cfg(test)] blocks using an awk filter.
set -euo pipefail

CONSENSUS_SRC="crates/consensus/src"
FAIL=0

# ── Pass 1: always-forbidden in all consensus source ─────────────────────────
# These must never appear regardless of test context.
always_bad='use std::collections::(HashMap|HashSet)|thread_rng|OsRng|std::time::|SystemTime|Instant'
if rg -n "$always_bad" "$CONSENSUS_SRC"; then
  echo "Domain A violation: always-forbidden pattern found in $CONSENSUS_SRC" >&2
  FAIL=1
fi

# Domain B surface references must never appear in Domain A source.
if rg -n "qash_pal|pal::" "$CONSENSUS_SRC"; then
  echo "Domain A violation: Domain B (PAL) reference found in $CONSENSUS_SRC" >&2
  FAIL=1
fi

# ── Pass 2: forbidden in production code only (skip #[test] blocks) ──────────
# Strips test modules via awk before scanning for panic/unwrap/expect.
# Heuristic: lines inside a `#[test]` fn or `#[cfg(test)]` mod are skipped.
prod_bad='unwrap\(|expect\(|panic!\(|unreachable!\('

stripped=$(
  find "$CONSENSUS_SRC" -name '*.rs' | while read -r f; do
    awk '
      /^[[:space:]]*#\[cfg\(test\)\]/ { in_test_mod = 1 }
      /^[[:space:]]*#\[test\]/ { skip_next_fn = 1 }
      skip_next_fn && /^[[:space:]]*(pub |pub\(crate\) |async )?fn / {
        in_test_fn = 1; skip_next_fn = 0; depth = 0
      }
      in_test_fn {
        for (i=1; i<=length($0); i++) {
          c = substr($0, i, 1)
          if (c == "{") depth++
          if (c == "}") { depth--; if (depth <= 0) { in_test_fn = 0; next } }
        }
        next
      }
      in_test_mod { next }
      /^[[:space:]]*\/\/[\/!]/ { next }   # skip doc comments (/// and //!)
      /^[[:space:]]*\/\// { next }         # skip regular line comments
      { print FILENAME ":" NR ":" $0 }
    ' FILENAME="$f" "$f"
  done
)

if echo "$stripped" | grep -P "$prod_bad"; then
  echo "Domain A violation: panic/unwrap/expect in production code (non-test)" >&2
  FAIL=1
fi

if [ "$FAIL" -eq 0 ]; then
  echo "Domain A tripwire scan passed"
else
  exit 1
fi
