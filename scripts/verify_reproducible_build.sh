#!/usr/bin/env bash
# verify_reproducible_build.sh — two-stage byte-identical build verification.
#
# Extends verify_two_stage_build.sh with SLSA provenance and SOURCE_DATE_EPOCH
# pinning for reproducible-build evidence. Produces a JSON provenance stub that
# can be submitted to a Sigstore/SLSA pipeline.
#
# Usage:
#   bash scripts/verify_reproducible_build.sh [--output-dir DIR]
#
# Outputs (written to OUTPUT_DIR, default: artifacts/reproducible-build/):
#   build-hashes.txt       — SHA-256 of each stage artifact
#   provenance.json        — SLSA provenance stub (build metadata only)
#   reproducible.txt       — PASS/FAIL summary

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

OUTPUT_DIR="${1:-artifacts/reproducible-build}"
mkdir -p "$OUTPUT_DIR"

BINARY="target/release/qash"
HASH_FILE="$OUTPUT_DIR/build-hashes.txt"
PROV_FILE="$OUTPUT_DIR/provenance.json"
SUMMARY="$OUTPUT_DIR/reproducible.txt"

# Pin SOURCE_DATE_EPOCH to the HEAD commit timestamp for reproducibility.
SOURCE_DATE_EPOCH="$(git log -1 --format=%ct HEAD)"
export SOURCE_DATE_EPOCH

COMMIT="$(git rev-parse HEAD)"
COMMIT_SHORT="$(git rev-parse --short=12 HEAD)"
BUILD_TS="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
RUST_VER="$(rustc --version 2>/dev/null || echo 'unknown')"
CARGO_VER="$(cargo --version 2>/dev/null || echo 'unknown')"

echo "SOURCE_DATE_EPOCH=$SOURCE_DATE_EPOCH (commit timestamp)"
echo "Commit: $COMMIT"
echo ""

echo "=== Stage 1 build ==="
cargo build --release --no-default-features
HASH1="$(sha256sum "$BINARY" | awk '{print $1}')"
echo "Stage 1: $HASH1"

echo ""
echo "=== Stage 2 build (clean qash package only) ==="
cargo clean -p qash
cargo build --release --no-default-features
HASH2="$(sha256sum "$BINARY" | awk '{print $1}')"
echo "Stage 2: $HASH2"

echo ""
{
    echo "# Reproducible Build Hashes"
    echo "# Generated: $BUILD_TS"
    echo "# Commit: $COMMIT"
    echo "# SOURCE_DATE_EPOCH: $SOURCE_DATE_EPOCH"
    echo "stage1=$HASH1"
    echo "stage2=$HASH2"
} > "$HASH_FILE"

if [ "$HASH1" = "$HASH2" ]; then
    RESULT="PASS"
    echo "PASS: artifact hashes match"
    echo "  $HASH1"
else
    RESULT="FAIL"
    echo "FAIL: artifact hashes differ between builds"
    echo "  Stage 1: $HASH1"
    echo "  Stage 2: $HASH2"
fi

# Write SLSA provenance stub (v0.2 schema subset).
# Full SLSA provenance requires a signed builder attestation from the CI
# environment (e.g., GitHub Actions OIDC token + Sigstore cosign).
# This stub records build inputs and can be submitted to a Sigstore pipeline
# once the CI attestation integration is complete.
cat > "$PROV_FILE" <<JSON
{
  "_type": "https://in-toto.io/Statement/v0.1",
  "predicateType": "https://slsa.dev/provenance/v0.2",
  "subject": [
    {
      "name": "qash",
      "digest": { "sha256": "$HASH1" }
    }
  ],
  "predicate": {
    "builder": { "id": "local-reproducible-build-script" },
    "buildType": "https://github.com/corp0ratestarts-cmd/qash/scripts/verify_reproducible_build.sh",
    "invocation": {
      "configSource": {
        "uri": "git+https://github.com/corp0ratestarts-cmd/qash",
        "digest": { "sha1": "$COMMIT" }
      },
      "environment": {
        "SOURCE_DATE_EPOCH": "$SOURCE_DATE_EPOCH",
        "RUST_VERSION": "$RUST_VER",
        "CARGO_VERSION": "$CARGO_VER"
      }
    },
    "metadata": {
      "buildStartedOn": "$BUILD_TS",
      "reproducible": $([ "$RESULT" = "PASS" ] && echo "true" || echo "false"),
      "stage1_sha256": "$HASH1",
      "stage2_sha256": "$HASH2",
      "note": "Stub only — full Sigstore/SLSA attestation requires CI OIDC integration"
    }
  }
}
JSON

{
    echo "Reproducible Build Verification"
    echo "================================"
    echo "Result:  $RESULT"
    echo "Commit:  $COMMIT_SHORT ($COMMIT)"
    echo "Hash:    $HASH1"
    echo "Written: $HASH_FILE"
    echo "         $PROV_FILE"
} | tee "$SUMMARY"

if [ "$RESULT" = "FAIL" ]; then
    exit 1
fi
