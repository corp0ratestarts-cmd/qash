# QASH Build Verification Guide

This document explains how to independently verify that a QASH binary was
produced from a specific source commit, was byte-identical across two
independent build stages, and (where supported) was compiled inside a
hardware-attested enclave.

---

## Two-stage byte-identical build

Every QASH release binary is built twice from the same source in the same CI
job. The two stage hashes are compared before the binary is published. This
guards against non-determinism introduced by incremental compilation artefacts
or build-system side channels.

**To reproduce locally:**

```sh
# Install the pinned toolchain (reads rust-toolchain.toml)
rustup toolchain install

# Stage 1
CARGO_INCREMENTAL=0 SOURCE_DATE_EPOCH=0 \
  cargo build --release --no-default-features
HASH1=$(sha256sum target/release/qash | awk '{print $1}')

# Stage 2 — clean only the top-level crate, rebuild
cargo clean -p qash
CARGO_INCREMENTAL=0 SOURCE_DATE_EPOCH=0 \
  cargo build --release --no-default-features
HASH2=$(sha256sum target/release/qash | awk '{print $1}')

[ "$HASH1" = "$HASH2" ] && echo "PASS" || echo "FAIL"
```

Alternatively, run the canonical verification script:

```sh
bash scripts/verify_reproducible_build.sh
```

---

## Sigstore Rekor transparency-log verification

Every build on `main` uploads the build hash to the [Sigstore Rekor](https://rekor.sigstore.dev)
append-only transparency log using keyless OIDC signing. This creates a
tamper-evident, publicly auditable log entry that binds:

```
binary SHA-256  ←→  source commit  ←→  GitHub Actions workflow identity
```

The Rekor bundle for each commit is archived under
`artifacts/attestations/rekor-bundle-<sha>.json` and also attached to the
`release-attestation-<sha>` CI artifact.

**To verify a binary against Rekor:**

```sh
# Prerequisites: cosign >= 2.0
# Install: https://github.com/sigstore/cosign/releases

bash scripts/verify_sigstore_attestation.sh \
  target/release/qash \
  <git-commit-sha>
```

The script checks:
- Binary SHA-256 matches the hash in the Rekor bundle.
- The OIDC certificate issuer is `https://token.actions.githubusercontent.com`.
- The certificate identity matches the QASH release workflow:
  `https://github.com/corp0ratestarts-cmd/qash/.github/workflows/release-attestation.yml@refs/heads/main`

If `cosign` is not installed, the script prints the binary hash and the
manual `cosign verify-blob` command for reference.

---

## Intel TDX enclave attestation (self-hosted runners)

When the CI job runs on a runner with Intel TDX support (`runner.cpu == 'tdx-enabled'`),
the build is additionally compiled *inside* a TDX enclave. The enclave produces:

| Artefact | Description |
|----------|-------------|
| `attestation-quote-<sha>.bin` | TDX quote binding the enclave measurement to the build |
| `build-measurement-<sha>.txt` | Enclave measurement (MRTD + RTMR registers) |
| `build-hash-<sha>.txt` | SHA-256 of the binary, produced inside the enclave |

These artefacts are archived alongside the Rekor bundle. The `attestation-quote`
can be verified independently using Intel's DCAP verification service or an
open-source TDX verifier.

On standard GitHub-hosted runners (no TDX), the enclave step is skipped and
the Sigstore Rekor binding is the primary tamper-evidence mechanism.

---

## Attestation artefact index

All attestation artefacts are committed to `artifacts/attestations/` by the
CI bot after each successful main-branch build:

```
artifacts/attestations/
  release-attestation-<sha>.txt   — human-readable manifest
  rekor-bundle-<sha>.json         — Sigstore Rekor bundle (machine-readable)
  (optional) rekor-bundle-<sha>.json   — present only for main-branch builds
```

---

## Trust model

| Mechanism | What it proves | Trust assumption |
|-----------|---------------|------------------|
| Two-stage byte-identical build | Binary is deterministic (no build-time randomness or incremental artefact pollution) | Honest CI runner |
| Sigstore Rekor (keyless OIDC) | Binary hash is bound to source commit and workflow identity at build time | Sigstore Rekor log is append-only and not compromised |
| Intel TDX quote | Build ran inside an attested enclave; enclave measurement matches expected value | Intel TDX firmware + DCAP infrastructure |

No single mechanism is sufficient alone. Together they implement defence in depth against a compromised CI runner (Rekor provides independent verification) and against supply-chain backdoors (TDX enclave measurement proves the build environment was not modified).
