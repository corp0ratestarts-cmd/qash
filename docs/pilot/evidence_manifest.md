# Pilot Evidence Manifest — QASH Pilot Baseline v0.2.1

**Tag:** `qash-pilot-baseline-v0.2.1`
**Date:** 2026-05-26
**Purpose:** Checklist for auditors and funders verifying the pilot evidence bundle.

---

## Artifacts

| File | Purpose | Classification | Expected SHA-256 |
|------|---------|----------------|-----------------|
| `public_commitments.bin` | Public WAL export — commitment hashes only | **Public** | run `sha256sum public_commitments.bin` |
| `replay.json` | Deterministic replay report over public commitments | **Public** | run `sha256sum replay.json` |
| `disclosure.bin` | Selective disclosure bundle for one receipt | **Public** | run `sha256sum disclosure.bin` |
| `claims_register.md` | Approved claims and blocked terms | **Public** | — |
| `post_merge_audit.md` | Post-merge privacy boundary audit | **Public** | — |

## Required Field Values in `replay.json`

| Field | Required Value |
|-------|---------------|
| `profile` | `"TX-MVP-ReceiptCommit"` |
| `profile_version` | `1` |
| `public_transcript_only` | `true` |
| `private_payloads_seen` | `false` |
| `status` | `"ok"` |

## Privacy Boundary Checks

These checks are run automatically by `scripts/build_pilot_evidence_bundle.sh`:

1. `public_commitments.bin` must not contain any of: `synthetic door alarm`, `synthetic offline incident`, `private body`, `INCIDENT`
2. `replay.json` must have `private_payloads_seen == false`
3. `replay.json` must have `profile_version` present
4. `disclosure.bin` must be non-empty

## Reproduction Instructions

Any third party can reproduce the evidence bundle:

```sh
# 1. Clone the repo at the tagged commit
git clone https://github.com/corp0ratestarts-cmd/qash
git checkout qash-pilot-baseline-v0.2.1

# 2. Run the demo
bash scripts/run_mvp_demo.sh --clean

# 3. Generate replay report
cargo run --bin qash-demo -- replay \
  --dir artifacts/mvp-demo/node-a \
  --report artifacts/mvp-demo/replay.json

# 4. Validate and package bundle
DEMO_DIR=artifacts/mvp-demo bash scripts/build_pilot_evidence_bundle.sh
```

## Out-of-Scope

- No production identity, payment, settlement, or custody operations
- No genesis-admitted validators
- No production ZK proofs or hardware attestation
- See `docs/mvp/claims_register.md` for the full approved claims boundary
