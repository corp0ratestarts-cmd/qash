# Determinism Certification Levels

This document defines the QASH determinism certification ladder and the evidence
required to claim each level.

## Levels

### L0 — Unit Determinism
- Scope: deterministic unit tests in Domain A (`crates/consensus`).
- Evidence: unit test vector outcomes are stable for repeated local runs under the
  pinned toolchain.
- Minimum claim: deterministic behavior for isolated functions/modules.

### L1 — Corpus Determinism (Per Commit)
- Scope: deterministic replay corpus for a specific commit SHA.
- Evidence: canonical replay vectors execute with stable pass/fail outcomes and
  stable state-root hashes for that commit.
- Minimum claim: deterministic behavior is demonstrated across the project corpus
  for each reviewed commit.

### L2 — Cross-ISA Determinism (`x86_64`, `aarch64`, `riscv64gc`)
- Scope: L1 corpus replay on all supported execution targets.
- Evidence: replay corpus and state-root hashes match across:
  - `x86_64-unknown-linux-gnu`
  - `aarch64-unknown-linux-gnu`
  - `riscv64gc-unknown-linux-gnu`
- Minimum claim: deterministic replay invariance across authorized ISAs.

### L3 — Reproducible Binary + Replay Attestation
- Scope: L2 plus reproducibility and provenance attestation.
- Evidence:
  - byte-identical reproducible build outputs
  - deterministic replay evidence bundle
  - signed provenance for the evidence bundle (cosign)
- Minimum claim: deterministic replay and binary provenance are jointly auditable.

## Evidence Bundle Location

Canonical bundle path (per commit):

`artifacts/certification/determinism/<sha>.json`

Signature/provenance sidecars:

- `artifacts/certification/determinism/<sha>.json.sig`
- `artifacts/certification/determinism/<sha>.json.bundle`
