#!/usr/bin/env bash
# Produces a reproducibility attestation manifest for the current build.
#
# Records: git commit, Rust toolchain, binary SHA-256, Cargo.lock hash,
# and per-crate source hashes. Outputs to stdout; pipe to a file for archiving.
#
# Usage:
#   ./scripts/attest_release.sh                        # local
#   ./scripts/attest_release.sh > attestation.txt      # capture
#
# In CI this is run after verify_two_stage_build.sh confirms byte-identical
# results, so the hash recorded here is trusted to be reproducible.
set -euo pipefail

BINARY="target/release/qash"
NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)
COMMIT=$(git rev-parse HEAD 2>/dev/null || echo "unknown")
BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")

echo "# QASH Release Attestation Manifest"
echo "# Generated:  $NOW"
echo "# Commit:     $COMMIT"
echo "# Branch:     $BRANCH"
echo ""

echo "## Toolchain"
rustc --version --verbose
echo ""

echo "## Build artifact"
if [ -f "$BINARY" ]; then
    BINARY_HASH=$(sha256sum "$BINARY" | awk '{print $1}')
    BINARY_SIZE=$(stat -c%s "$BINARY")
    echo "binary:  $BINARY"
    echo "sha256:  $BINARY_HASH"
    echo "size:    $BINARY_SIZE bytes"
else
    echo "binary:  NOT FOUND — run: cargo build --release --no-default-features"
    exit 1
fi
echo ""

echo "## Source integrity"
LOCK_HASH=$(sha256sum Cargo.lock | awk '{print $1}')
CONSENSUS_HASH=$(find crates/consensus/src -name '*.rs' | sort | xargs sha256sum | sha256sum | awk '{print $1}')
TOML_HASH=$(sha256sum GENESIS_CONSTANTS.toml | awk '{print $1}')
echo "Cargo.lock sha256:             $LOCK_HASH"
echo "crates/consensus/src sha256:   $CONSENSUS_HASH"
echo "GENESIS_CONSTANTS.toml sha256: $TOML_HASH"
echo ""

echo "## Dependency version list (from Cargo.lock)"
grep -E '^(name|version) = ' Cargo.lock \
    | paste - - \
    | sed 's/name = "//; s/" version = "/  /; s/"//' \
    | column -t
