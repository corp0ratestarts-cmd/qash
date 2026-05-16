#!/usr/bin/env bash
# Verifies two sequential release builds of the qash binary produce identical
# SHA-256 hashes. This is a necessary condition for cross-ISA determinism (TH-7).
#
# Stage 1: build normally.
# Stage 2: clean only the qash package (keeps dependency caches) and rebuild.
# If the hashes differ, the build is not reproducible on this host and must not
# be used as a genesis artifact.
set -euo pipefail

BINARY="target/release/qash"

echo "=== Stage 1 build ==="
cargo build --release --no-default-features
HASH1=$(sha256sum "$BINARY" | awk '{print $1}')
echo "Stage 1: $HASH1"

echo ""
echo "=== Stage 2 build (clean qash package only) ==="
cargo clean -p qash
cargo build --release --no-default-features
HASH2=$(sha256sum "$BINARY" | awk '{print $1}')
echo "Stage 2: $HASH2"

echo ""
if [ "$HASH1" = "$HASH2" ]; then
    echo "PASS: artifact hashes match"
    echo "  $HASH1"
else
    echo "FAIL: artifact hashes differ between builds"
    echo "  Stage 1: $HASH1"
    echo "  Stage 2: $HASH2"
    exit 1
fi
