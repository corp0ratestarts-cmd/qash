# QASH Compliance Evidence Matrix

This document separates compliance evidence into four tiers:
- **Protocol-level**: what the CI pipeline proves about the consensus protocol
- **Deployment-level**: what an operator must prove for a specific deployment
- **Operator-level**: what individual node operators attest
- **Jurisdiction-specific**: controls that depend on local law/regulation

CI does NOT cover deployment, operator, or jurisdiction controls. Mixing
these tiers is a common source of false compliance claims.

---

## Protocol-Level Evidence (CI-provable)

These claims are backed by reproducible CI artifacts on every commit.

| Control | Evidence | CI Gate |
|---------|----------|---------|
| Deterministic replay across x86\_64/aarch64/riscv64gc | `artifacts/replay_equivalence/` | `cross-verify` |
| Zero Admitted Coq theorems (22+ theorems) | `proof-hashes.txt`; `proofs/` | `proofs` |
| Domain A / Domain B hard partition | `scripts/check_domain_a_tripwires.sh` | `QASH domain boundary tripwires` |
| Zero-persistence PAL (no raw tx/peer/graph in WAL) | `scripts/check_zero_persistence_boundary.sh` | `zero-persistence-boundary` |
| Reproducible builds (two-stage, `SOURCE_DATE_EPOCH=0`) | `scripts/verify_two_stage_build.sh` | `test-determinism` |
| Supply chain: license + advisory clean | `deny.toml` | `supply-chain` |
| PQC cascade: SHA3-256, SM3, Dilithium5 (planned) | `GENESIS_CONSTANTS.toml`; `crates/consensus/src/cascade.rs` | (module tests) |
| Receipt privacy: no public-network disclosure | `crates/pal/tests/receipt_privacy_negative.rs` | `test-determinism` |
| Fuzz coverage: 6 targets × 30s | fuzz harnesses | `fuzz-smoke` |
| 70k interpreter conformance assertions | `crates/consensus/tests/interpreter_conformance.rs` | `test-determinism` |
| Lyapunov confluence / Church-Rosser | `proofs/composition/lyapunov_confluence.v` | `proofs`; `test-determinism` (LC-1/LC-2 gate) |
| EffectToken boundary schema (CT-1..CT-4) | `proofs/capability/cap_token_schema.v` | `proofs` |

---

## Deployment-Level Evidence (Operator Must Provide)

These controls depend on how a specific deployment is configured. The
QASH protocol CI makes no claim about them.

| Control | Operator Evidence Required |
|---------|--------------------------|
| Network transport security (TLS 1.3 / mutual auth) | TLS configuration + cert issuance records |
| Validator key management (HSM, TPM, or software) | Key custody policy + attestation logs |
| Infrastructure access control | IAM policy + audit logs |
| Incident response plan | Written IR plan + tabletop exercise records |
| Data residency / geographic isolation | Deployment topology diagram + provider agreements |
| Backup and recovery for WAL | Backup policy + tested restore records |

---

## Operator-Level Evidence (Individual Node Operator)

Each node operator running qash-pal is responsible for:

| Control | Evidence |
|---------|----------|
| Hardware attestation configuration | Local attestation policy (attestation.rs LocalAttestationVerifier) |
| Local key shredding policy | Shred commitment records from ReceiptVault |
| Software update / patch policy | Deployment runbook |
| Audit log retention | Log archive policy |

---

## Jurisdiction-Specific Controls

These are not covered by QASH protocol CI and require separate legal / compliance analysis:

| Jurisdiction | Applicable Regulations | Status |
|--------------|----------------------|--------|
| EU | GDPR Art. 35 DPIA (key shredding, right-to-erasure via shred commitment) | See `docs/compliance/dpia.md` (planned) |
| USA (federal) | FIPS 140-3 module certification | Domain B only; see `docs/compliance/fips_compliance.md` |
| USA (DoD) | CMMC 2.0 Level 2 | Not assessed |
| Canada | PIPEDA | Not assessed |
| UK | UK GDPR | Follows EU GDPR analysis with post-Brexit divergences |

---

## What CI Does NOT Prove

- That a specific deployment is secure (CI runs on the codebase, not a live system)
- That production keys are handled safely (Domain B is non-deterministic by design)
- That epoch timing constants (500ms / simulation-only) are suitable for any specific deployment
- That the genesis constants are appropriate for a production network (genesis is not yet locked)
- FIPS 140-3 module certification (requires separate testing lab evaluation)
- GDPR compliance for any specific data processor
