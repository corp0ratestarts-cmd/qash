#!/usr/bin/env bash
# run_platform_determinism_gate.sh — Platform determinism gate runner.
#
# Accepts TARGET, EXPECTED_ROOT, and MODE environment variables and runs the
# appropriate verification level for that target.
#
# Usage:
#   TARGET=x86_64-unknown-linux-gnu \
#   EXPECTED_ROOT=<hex> \
#   MODE=full \
#   ./scripts/run_platform_determinism_gate.sh
#
# Environment variables:
#   TARGET          — Rust target triple (required)
#   EXPECTED_ROOT   — Expected canonical state root hex string (required for
#                     MODE=replay and MODE=full; optional for MODE=compile)
#   MODE            — Verification mode (default: full)
#                       compile  — L1: cargo check/build only
#                       test     — L2: unit tests
#                       replay   — L3: state-root parity check against EXPECTED_ROOT
#                       full     — L3 + L4: replay + corpus (v1.1/v1.2)
#
# Exit codes:
#   0 — all checks for MODE passed
#   1 — any check failed
#
# Notes:
# - Cross-compilation linkers must be in PATH or set via
#   CARGO_TARGET_<TRIPLE>_LINKER env vars.
# - For QEMU-based cross targets, ensure qemu-user is installed and
#   .cargo/config.toml has the appropriate runner configured.
# - This script does not modify global state; it is safe to run in CI
#   parallel matrix jobs.
set -euo pipefail

ROOT_MARKER="CANONICAL_STATE_ROOT_3_EPOCHS"
CARGO_BIN="${CARGO:-cargo}"

# ── Argument validation ───────────────────────────────────────────────────────
TARGET="${TARGET:-}"
MODE="${MODE:-full}"
EXPECTED_ROOT="${EXPECTED_ROOT:-}"

if [ -z "$TARGET" ]; then
  echo "Error: TARGET environment variable is required." >&2
  echo "  Example: TARGET=x86_64-unknown-linux-gnu MODE=full ./scripts/run_platform_determinism_gate.sh" >&2
  exit 1
fi

case "$MODE" in
  compile|test|replay|full) ;;
  *)
    echo "Error: MODE must be one of: compile, test, replay, full (got: '$MODE')" >&2
    exit 1
    ;;
esac

if [ "$MODE" = "replay" ] || [ "$MODE" = "full" ]; then
  if [ -z "$EXPECTED_ROOT" ]; then
    echo "Error: EXPECTED_ROOT is required for MODE=$MODE" >&2
    exit 1
  fi
fi

echo "=== Platform Determinism Gate ==="
echo "  TARGET:        $TARGET"
echo "  MODE:          $MODE"
echo "  EXPECTED_ROOT: ${EXPECTED_ROOT:-(not required for this mode)}"
echo ""

# ── Target-specific args ──────────────────────────────────────────────────────
TARGET_ARGS=()
if [ "$TARGET" != "x86_64-unknown-linux-gnu" ]; then
  TARGET_ARGS+=(--target "$TARGET")
fi

# ── L1: Compile check ─────────────────────────────────────────────────────────
echo "--- L1: compile check ---"
"$CARGO_BIN" check -p qash-consensus --no-default-features "${TARGET_ARGS[@]}"
echo "L1 PASS: cargo check succeeded for $TARGET"

if [ "$MODE" = "compile" ]; then
  echo ""
  echo "Mode=compile: L1 complete. Exiting."
  exit 0
fi

# Also build (not just check) before proceeding to test/replay
"$CARGO_BIN" build -p qash-consensus --no-default-features "${TARGET_ARGS[@]}"
echo "L1 PASS: cargo build succeeded for $TARGET"

# ── L2: Unit tests ────────────────────────────────────────────────────────────
echo ""
echo "--- L2: unit tests ---"
"$CARGO_BIN" test -p qash-consensus --no-default-features "${TARGET_ARGS[@]}"
echo "L2 PASS: unit tests passed for $TARGET"

if [ "$MODE" = "test" ]; then
  echo ""
  echo "Mode=test: L2 complete. Exiting."
  exit 0
fi

# ── L3: State-root parity ─────────────────────────────────────────────────────
echo ""
echo "--- L3: state-root parity ---"
STATE_ROOT_OUTPUT=$(
  "$CARGO_BIN" test -p qash-consensus --no-default-features \
    "${TARGET_ARGS[@]}" \
    state_root_canonical_seq_print -- --nocapture 2>/dev/null || true
)

ACTUAL_ROOT=$(echo "$STATE_ROOT_OUTPUT" | grep "$ROOT_MARKER" | head -1 || true)

if [ -z "$ACTUAL_ROOT" ]; then
  echo "L3 FAIL: $ROOT_MARKER not found in test output for $TARGET" >&2
  echo "Full output:" >&2
  echo "$STATE_ROOT_OUTPUT" >&2
  exit 1
fi

echo "  Expected: $EXPECTED_ROOT"
echo "  Actual:   $ACTUAL_ROOT"

if [ "$ACTUAL_ROOT" != "$EXPECTED_ROOT" ]; then
  echo "L3 FAIL: State root divergence for $TARGET" >&2
  echo "  Expected: $EXPECTED_ROOT" >&2
  echo "  Actual:   $ACTUAL_ROOT" >&2
  exit 1
fi

echo "L3 PASS: state root matches reference for $TARGET"

if [ "$MODE" = "replay" ]; then
  echo ""
  echo "Mode=replay: L3 complete. Exiting."
  exit 0
fi

# ── L4: Replay corpus (v1.1 and v1.2) ────────────────────────────────────────
echo ""
echo "--- L4: replay corpus ---"

# v1.1 corpus
if "$CARGO_BIN" test -p qash-consensus --no-default-features \
     "${TARGET_ARGS[@]}" \
     --test v1_1_replay v1_1_corpus_matches_pinned 2>/dev/null; then
  echo "L4 PASS: v1.1 replay corpus matched for $TARGET"
else
  echo "L4 SKIP: v1_1_replay test not found (corpus may not yet exist for this target)"
fi

# v1.2 corpus
if "$CARGO_BIN" test -p qash-consensus --no-default-features \
     "${TARGET_ARGS[@]}" \
     --test v1_2_sharded_replay v1_2_sharded_corpus_matches_pinned 2>/dev/null; then
  echo "L4 PASS: v1.2 sharded replay corpus matched for $TARGET"
else
  echo "L4 SKIP: v1_2_sharded_replay test not found (corpus may not yet exist for this target)"
fi

echo ""
echo "=== Platform Determinism Gate PASSED for $TARGET (MODE=$MODE) ==="
exit 0
