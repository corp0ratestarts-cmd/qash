#!/usr/bin/env bash
# replay_test.sh — Run the v1.1 replay corpus on all authorized ISAs.
#
# Usage:
#   ./scripts/replay_test.sh [--native-only]
#
# Runs the `v1_1_corpus_matches_pinned` test which asserts the 50-epoch
# state_root sequence in tests/vectors/vectors.v1.1.json is produced
# bit-identically.  With --native-only (or when cross-compilation tools
# are absent) only the native x86_64 target is tested.
#
# For cross-ISA CI: call without --native-only in the platform-determinism
# GitHub Actions workflow, which provides QEMU user-static binfmt_misc
# entries for aarch64 and riscv64gc.
#
# Exit code: 0 on success, 1 on any failure.

set -euo pipefail

NATIVE_ONLY=false
if [[ "${1:-}" == "--native-only" ]]; then
    NATIVE_ONLY=true
fi

TARGETS=("x86_64-unknown-linux-gnu")
if ! $NATIVE_ONLY; then
    TARGETS+=(
        "aarch64-unknown-linux-gnu"
        "riscv64gc-unknown-linux-gnu"
    )
fi

PASS=0
FAIL=0

for target in "${TARGETS[@]}"; do
    echo "=== replay_test: $target ==="

    if [[ "$target" == "x86_64-unknown-linux-gnu" ]]; then
        # Native: run directly.
        if cargo test -p qash-consensus --no-default-features \
               --test v1_1_replay v1_1_corpus_matches_pinned \
               2>&1; then
            echo "  PASS $target"
            ((PASS++)) || true
        else
            echo "  FAIL $target"
            ((FAIL++)) || true
        fi
    else
        # Cross: requires cross (https://github.com/cross-rs/cross) and
        # the target rust toolchain to be installed.
        if ! command -v cross &>/dev/null; then
            echo "  SKIP $target (cross not installed)"
            continue
        fi
        if cross test -p qash-consensus --no-default-features \
               --target "$target" \
               --test v1_1_replay v1_1_corpus_matches_pinned \
               2>&1; then
            echo "  PASS $target"
            ((PASS++)) || true
        else
            echo "  FAIL $target"
            ((FAIL++)) || true
        fi
    fi
done

echo ""
echo "replay_test summary: $PASS passed, $FAIL failed"
if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
