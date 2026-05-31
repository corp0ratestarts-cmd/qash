# Common Criteria Security Target — QASH

**Document type:** CC EAL4+ Security Target (design-phase draft)  
**Date:** 2026-05-30  
**Status:** Design-phase draft — not evaluated or certified  
**CC Standard:** ISO/IEC 15408-1:2022 (Common Criteria v3.1 Rev. 5)  
**Assurance Level Target:** EAL4 augmented (EAL4+) — design-phase target only, not a certified claim

> **Claim boundary:** This document is a design-phase draft Security Target.
> Formal CC certification requires engagement with a NIST-accredited or
> CCRA-recognised evaluation laboratory. "EAL4+" is the target assurance level,
> not a certified claim.

---

## Status Vocabulary

The following status labels are used throughout this document:

| Status label | Meaning |
|---|---|
| N/A | Not applicable to this repo |
| Internal alignment | Code/design follows the standard's approach; no external assessment |
| Implementation complete / self-tested | Implemented with CI KATs; no external validation |
| Externally certified | Formal certificate or report exists — none currently |

---

## 1. TOE Identification

| Field | Value |
|-------|-------|
| TOE Name | QASH Consensus Engine |
| TOE Version | Pre-genesis RC (v0.x; version locked at genesis) |
| TOE Type | Deterministic consensus and incident-log commitment substrate |
| Developer | corp0ratestarts-cmd |
| Sponsor | corp0ratestarts-cmd |

---

## 2. TOE Overview

QASH is a post-quantum, zero-governance, deterministic consensus engine
designed for offline-operable, jurisdiction-neutral, replay-deterministic
incident-log commitment. The Target of Evaluation (TOE) consists of:

- **Domain A (TOE boundary — primary):** `crates/consensus/` — `no_std`,
  no-`unsafe`, deterministic state-transition logic. This is the proof-eligible
  consensus core; all arithmetic is checked and overflow-absorbing.
- **Domain B (TOE boundary — selected crypto):** `crates/pal/src/crypto/`
  selected cryptographic primitives: ML-KEM-768 (`kem.rs`), HMAC-DRBG (`drbg.rs`),
  key shredding (`privacy/erasure.rs`). These Domain B components are included
  in the TOE boundary for the cryptographic SFRs (FCS_CKM.1, FCS_RBG_EXT.1,
  FCS_CKM.4, FDP_RIP.1). All other Domain B infrastructure is TOE environment.
- **TOE environment:** Remaining `crates/pal/` modules and the hosted binary —
  network transport, attestation, threshold signing, ZK backend, and operator
  tooling. These are outside the formal TOE claim boundary.

The TOE does not include network infrastructure, operator key management
systems, or the hardware platform on which it executes.

---

## 3. TOE Security Environment

### 3.1 Assets

| Asset | Sensitivity | Location |
|-------|-------------|----------|
| Consensus state root | High — integrity-critical | Domain A `EpochState.state_root` |
| Epoch transition correctness | High — safety-critical | Domain A `advance_epoch()` |
| Receipt root commitments | High — audit-critical | Domain A `EpochState.receipt_root` |
| Encrypted receipt material | High — confidentiality | Domain B WAL (outside TOE boundary) |
| Capability tokens | High — boundary integrity | Domain A `CapToken<T>` |
| Formal proof objects | High — assurance evidence | `proofs/*.vo` |

### 3.2 Threats

| ID | Threat | Source |
|----|--------|--------|
| T.FORGE | Adversary forges a valid state root without executing correct transitions | External attacker |
| T.CORRUPT | Adversary corrupts consensus state via Domain B value injection | Malicious PAL code |
| T.REPLAY | Adversary replays a past state transition as a new one | External attacker |
| T.HALT-BYPASS | Adversary bypasses absorbing halt, enabling continued execution in invalid state | External or internal attacker |
| T.FLOAT | Adversary introduces floating-point non-determinism via type confusion | Developer error / compiler bug |
| T.OVERFLOW | Adversary triggers unchecked arithmetic overflow to produce incorrect state | External attacker |

### 3.3 Organisational Security Policies

| ID | Policy |
|----|--------|
| P.DETERMINISM | The TOE shall produce bit-identical state roots on all authorised ISAs (x86_64, aarch64, riscv64gc) |
| P.NO-FLOAT | The TOE shall contain no floating-point types in consensus state or arithmetic |
| P.ABSORB | All arithmetic overflow shall trigger an absorbing halt with no silent wrap-around |
| P.DOMAIN-ISOLATION | Domain B values shall not flow into Domain A computations |
| P.PROOF-COVERAGE | All safety-critical properties shall be verified by Coq proofs with zero Admitted markers |

---

## 4. Security Objectives

### 4.1 TOE Security Objectives

| ID | Objective | Rationale |
|----|-----------|-----------|
| O.STATE-INTEGRITY | The TOE shall ensure that the state root uniquely encodes the current consensus state | Counters T.FORGE, T.REPLAY |
| O.HALT-ABSORBING | The TOE shall ensure that a halted state is terminal and irreversible | Counters T.HALT-BYPASS |
| O.DOMAIN-ISOLATION | The TOE shall enforce that Domain B values never influence Domain A computations | Counters T.CORRUPT |
| O.ARITHMETIC-SAFETY | The TOE shall ensure all arithmetic is checked; overflow triggers absorbing halt | Counters T.OVERFLOW |
| O.DETERMINISM | The TOE shall be replay-deterministic across all authorised ISAs | Supports P.DETERMINISM |
| O.NO-FLOAT | The TOE shall contain no floating-point types in the consensus path | Counters T.FLOAT, supports P.NO-FLOAT |

### 4.2 Security Objectives for the Operational Environment

| ID | Objective |
|----|-----------|
| OE.KEY-MGMT | The operational environment shall manage private keys outside the TOE boundary |
| OE.PLATFORM | The operational environment shall provide a hardware platform with supported ISA |
| OE.ADMIN | Administrators shall not modify `GENESIS_CONSTANTS.toml` after genesis lock |

---

## 5. Security Functional Requirements (SFRs)

### FCS — Cryptographic Support

| SFR | Description | Implementation | CI evidence |
|-----|-------------|----------------|-------------|
| FCS_CKM.1 | Cryptographic key generation — ML-KEM-768 | `crates/pal/src/crypto/kem.rs::MlKem768KeyPair::from_seed` (Domain B, `pqc` feature) | `cavp-kat`: `cavp_ml_kem_768`; `tests/cavp/ml_kem_768.json` |
| FCS_CKM.4 | Cryptographic key destruction — ZeroizeOnDrop | `crates/pal/src/privacy/erasure.rs::shred_key()` | `zero-persistence-boundary`; `erasure_workflow` integration test |
| FCS_COP.1(hash) | SHA3-256 state root and receipt root hashing | `crates/consensus/src/hash.rs::sha3_256` (Domain A) | `cavp-kat`: `cavp_sha3_256`; `tests/cavp/sha3_256.json` |
| FCS_COP.1(drbg) | HMAC-DRBG per NIST SP 800-90A | `crates/pal/src/crypto/drbg.rs::FipsDrbg` (Domain B) | `cavp-kat`: `cavp_hmac_sha256`; `tests/cavp/hmac_sha256.json` |
| FCS_RBG_EXT.1 | Random bit generation — FIPS-aligned HMAC-DRBG | `crates/pal/src/crypto/drbg.rs::FipsDrbg` (Domain B) | `cavp-kat`: `cavp_hmac_sha256`; POST `post_hmac_drbg` (`fips-post` feature) |

### FPT — Protection of the TSF

| SFR | Description | Implementation | CI evidence |
|-----|-------------|----------------|-------------|
| FPT_TST.1 | TSF testing — cargo test + Kani harnesses | `cargo test --workspace`, `scripts/run_kani_consensus.sh` | `test-determinism`, `kani-advisory` CI jobs |
| FPT_FLS.1 | Failure with preservation of secure state — absorbing halt | `crates/consensus/src/transition.rs::advance_epoch` (halt-flag guard) | `TH6_halt_terminal`, `TH6_halt_irreversible` (`proofs/safety/absorbing_halt.v`) |
| FPT_ITT.1 | Internal TSF data transfer integrity — domain isolation | `crates/consensus/src/domain.rs`, `CapToken<T>` | `domain-a-tripwires`, `Domain A/B full boundary scan` |

### FDP — User Data Protection

| SFR | Description | Implementation | CI evidence |
|-----|-------------|----------------|-------------|
| FDP_IFC.1 | Subset information flow control — Domain A/B boundary | `crates/consensus/src/capability.rs::validate_capability()` | `domain-a-tripwires` CI; `domain_crossing_is_explicit` Coq lemma |
| FDP_IFF.1 | Simple security attributes — CapToken schema | `proofs/capability/cap_token_schema.v` | `proofs` CI job (Coq compile) |
| FDP_RIP.1 | Subset residual information protection — key shredding | `crates/pal/src/privacy/erasure.rs::process_erasure_request` | `zero-persistence-boundary`; `erasure_workflow` integration test |

---

## 6. Security Assurance Requirements (SARs)

Target assurance level: **EAL4 augmented**.

| SAR | Description | Evidence |
|-----|-------------|---------|
| ADV_ARC.1 | Security architecture description | This document §2 TOE boundary, CLAUDE.md Domain A/B partition |
| ADV_FSP.4 | Complete functional specification | `docs/spec/`, module-level doc comments in `crates/consensus/src/` |
| ADV_IMP.1 | Implementation representation | Full source in `crates/consensus/`; `crates/pal/` |
| ADV_TDS.3 | Basic modular design | Workspace layout, `docs/adr/`, `docs/implementation_order.md` |
| AGD_OPE.1 | Operational user guidance | `GENESIS_CONSTANTS.toml` comments, `docs/` |
| AGD_PRE.1 | Preparative procedures | `scripts/verify_rust_toolchain.sh`, toolchain pin in `rust-toolchain.toml` |
| ATE_COV.2 | Analysis of coverage | `proofs/COVERAGE.md` — 43 PROVED, 7 CI-VERIFIED, 3 AXIOM, 2 PLACEHOLDER (56 total) |
| ATE_DPT.1 | Testing: basic design | `crates/consensus/tests/`, `crates/pal/tests/` |
| ATE_FUN.1 | Functional testing | `cargo test --workspace`, `scripts/verify_two_stage_build.sh` |
| ATE_IND.2 | Independent testing — sample | Kani harnesses (`scripts/run_kani_consensus.sh`) |
| AVA_VAN.3 | Focused vulnerability analysis | `codeql` advisory CI job; `scripts/check_domain_a_tripwires.sh` |
| ALC_CMC.4 | Production support, acceptance procedures, and automation | GitHub Actions CI (`ci.yml`), branch protection ruleset |
| ALC_CMS.4 | Problem tracking CM coverage | GitHub Issues; `COVERAGE.md` open obligations |
| ALC_DVS.1 | Identification of security measures | `GENESIS_CONSTANTS.toml` append-only policy; Domain A/B partition |
| ALC_TAT.1 | Well-defined development tools | `rust-toolchain.toml` pinned toolchain; `cargo deny check` |

---

## 7. TOE Summary Specification

### 7.1 State Root Integrity (O.STATE-INTEGRITY)

The state root is computed by a deterministic fold of SHA3-256 over the full
`EpochState` encoding. The encoding is injective (proved: `TH1_encode_state_injective`
in `proofs/contractivity/encode_injectivity.v`). The full state root
uniqueness under halt is proved: `TH8_full_uniqueness`.

### 7.2 Absorbing Halt (O.HALT-ABSORBING)

`advance_epoch` checks the `halt_flag` before any state mutation. If set,
the function returns immediately with the existing state unchanged.
Proved irreversible: `TH6_halt_terminal`, `TH6_halt_irreversible` in
`proofs/safety/absorbing_halt.v`.

### 7.3 Domain Isolation (O.DOMAIN-ISOLATION)

Domain B values enter Domain A only through `CapToken<T>` unwrapping.
The CapToken schema is formally proved in `proofs/capability/cap_token_schema.v`:
- `domain_crossing_is_explicit`: `into_inner()` is the sole observation path
- `cap_token_schema_injective`: CapToken is a transparent newtype; no hidden metadata

The CI job `domain-a-tripwires` (`scripts/check_domain_a_tripwires.sh`) rejects
`HashMap`, `f32`/`f64`, and `usize`/`isize` in Domain A state fields at every push.

### 7.4 Arithmetic Safety (O.ARITHMETIC-SAFETY)

All arithmetic in `crates/consensus/` uses checked operations with overflow
routing to `Halt::absorbing_reset()`. The Lyapunov potential decreases monotonically
to zero (proved: `TH3c_finalize_zero`). Cascade health bounds are proved:
`ch_t_upper_bound`, `ch_term_admissible` in `proofs/cascade/cascade_health_bounded.v`.

### 7.5 Cross-ISA Determinism (O.DETERMINISM)

Bit-identical state roots on x86_64, aarch64, and riscv64gc are enforced by:
- CI cross-compile matrix (`platform-determinism.yml`)
- `scripts/verify_two_stage_build.sh` two-stage deterministic build pipeline
- v1.1 replay corpus pinned in `tests/vectors/vectors.v1.1.json`

---

## 8. Rationale

### 8.1 SFR–Objective Mapping

| Objective | SFRs |
|-----------|------|
| O.STATE-INTEGRITY | FCS_COP.1(hash), FDP_IFF.1 |
| O.HALT-ABSORBING | FPT_FLS.1 |
| O.DOMAIN-ISOLATION | FDP_IFC.1, FDP_IFF.1, FPT_ITT.1 |
| O.ARITHMETIC-SAFETY | FPT_FLS.1, FPT_TST.1 |
| O.DETERMINISM | FPT_TST.1 |
| O.NO-FLOAT | FPT_TST.1, (AVA_VAN.3 via tripwire CI) |

### 8.2 Proof Coverage Adequacy

The 43 proved Coq theorems in `proofs/COVERAGE.md` cover all safety-critical
properties listed in §5. There are also 7 CI-VERIFIED properties enforced by
non-Coq CI (determinism replay, cascade KATs, and versioning/health tracking).
The 3 AXIOM entries (`AX2-refinement`, `GRC-7-7-v2`, `TH-10-cascade-collision`)
and 2 PLACEHOLDER entries are documented with non-vacuous typed axioms and
their v1.0 claim-boundary classifications. No MISSING entries exist (56 total).

---

## 9. Evaluator Build and Verification Instructions

To reproduce the TOE build and run the evidence-generating test suite:

```sh
# Prerequisites: Rust toolchain pinned in rust-toolchain.toml
rustup toolchain install

# 1. Clean build (no default features — Domain A only)
cargo build --no-default-features

# 2. Full test suite (consensus core + PAL)
cargo test --workspace --no-default-features

# 3. Verify genesis hash (computed == recorded)
bash scripts/verify_genesis_hash.sh

# 4. Verify deterministic two-stage build pipeline
bash scripts/verify_two_stage_build.sh

# 5. Run CAVP KAT vectors
cargo test -p qash-consensus --no-default-features -- hash::tests::cavp_sha3_256 --nocapture
cargo test -p qash-pal -- crypto::drbg::tests::cavp_hmac_sha256 --nocapture
cargo test -p qash-pal --features pqc -- crypto::kem::tests::cavp_ml_kem_768 --nocapture

# 6. Run FIPS POST self-tests
cargo test -p qash-pal --features fips-post -- post --nocapture

# 7. Compile Coq proofs (requires coq >= 8.18)
cd proofs && make all

# 8. Inspect proof coverage
cat proofs/COVERAGE.md   # 43 PROVED, 7 CI-VERIFIED, 3 AXIOM, 2 PLACEHOLDER (56 total)
```

CI artifacts (proof hashes, SBOM, bench output) are uploaded on each push to
`main` — see `.github/workflows/ci.yml` for the full 65-job pipeline.

---

## 10. Open Items (Pre-Certification)

The following items must be resolved before a CC evaluation can proceed:

| ID | Item | Status |
|----|------|--------|
| CC-01 | Engage CMVP/CCRA lab for EAL4+ evaluation | Not started (pre-genesis; external process) |
| CC-02 | Strengthen AX2-refinement (Coq ↔ Rust observational equivalence) | ACCEPTED axiom with 10 CI vectors; post-v1.0 enhancement |
| CC-03 | Discharge TH-10 cascade collision resistance | PLACEHOLDER (post-genesis migration item; SHA3-256 is the active v1.0 state-root primitive) |
| CC-04 | Complete production PAL transport (Track 4) | In progress (scaffolds exist; real networking is post-genesis) |
| CC-05 | Genesis lock and `v1.0-reference` tag | Blocked on Phase 1-D (human PDF traceability review) |
| CC-06 | Normative PDF specification committed to `spec/pdf/` | ✅ Done — `spec/pdf/QASH_Spec_v1.0.pdf` committed (provisional until 1-D review) |
| CC-07 | Traceability artifact reconciliation | Pending Phase 1-D (manual PDF review — human gate) |
