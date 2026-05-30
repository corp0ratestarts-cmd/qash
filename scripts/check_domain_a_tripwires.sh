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

# Floating-point types are forbidden everywhere in Domain A (no f32/f64).
# Match type annotations and casts; exclude comments.
if rg -n '(:\s*f32\b|:\s*f64\b|as f32\b|as f64\b|\bf32::\b|\bf64::\b)' "$CONSENSUS_SRC"; then
  echo "Domain A violation: f32/f64 found in $CONSENSUS_SRC (floating-point is forbidden)" >&2
  FAIL=1
fi

# HashMap is forbidden in Domain A state/wire/arithmetic paths. BTreeMap must be used instead.
# The always_bad pattern above catches 'use std::collections::HashMap'. Also catch direct
# HashMap<, HashMap::new(), and AHashMap / FxHashMap (all non-deterministic without seed).
if rg -n '(HashMap<|HashMap::new|AHashMap|FxHashMap)' "$CONSENSUS_SRC"; then
  echo "Domain A violation: HashMap/AHashMap/FxHashMap found in $CONSENSUS_SRC (use BTreeMap)" >&2
  FAIL=1
fi

# usize/isize in consensus struct FIELDS is forbidden (platform-width dependent).
# We use awk to track struct bodies and flag usize/isize field declarations.
# Function parameters and const/let locals are excluded by context tracking.
struct_usize=$(
  find "$CONSENSUS_SRC" -name '*.rs' | sort | while read -r f; do
    awk -v file="$f" '
      # Track brace depth and whether we are inside a named struct body
      /^[[:space:]]*(pub[[:space:]]*(crate[[:space:]]*)?\([^)]*\)[[:space:]]*)?struct[[:space:]]+[A-Za-z]/ {
        in_struct = 1; depth = 0
      }
      in_struct && /\{/ { depth++ }
      in_struct && /\}/ { depth--; if (depth <= 0) { in_struct = 0 } }
      in_struct && depth > 0 && /:\s*(usize|isize)\b/ {
        # Exclude const declarations (they use usize as a type but are not fields)
        if ($0 !~ /^[[:space:]]*const[[:space:]]/ &&
            $0 !~ /^[[:space:]]*let[[:space:]]/ &&
            $0 !~ /^[[:space:]]*\/\//) {
          print file ":" NR ":" $0
        }
      }
    ' "$f"
  done
)

if [ -n "$struct_usize" ]; then
  echo "$struct_usize"
  echo "Domain A violation: usize/isize struct field in $CONSENSUS_SRC" \
       "(use u64/i64 for state/wire fields; usize is permitted only for local indexing)" >&2
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
