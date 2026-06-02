# v1.0 Axiom and Release-Boundary Assumptions

**Date:** 2026-06-01
**Status:** Wave 2 (PR #228) — complete axiom classification for genesis-candidate gate.

This document formally classifies every axiom and placeholder in the QASH proof
corpus by its v1.0 release-boundary status. It is the authoritative companion to
`proofs/COVERAGE.md`.

---

## Classification taxonomy

| Class | Meaning |
|-------|---------|
| **ACCEPTED-CRYPTOGRAPHIC** | Standard hardness assumption (collision resistance, PRF security, AU bound). Acceptable for genesis lock; assumed computationally, not proved within the system. |
| **ACCEPTED-COMPILER** | Assumed-correct toolchain / Rust semantics. Explicitly named AX-1 and AX-2 in PDF §3.11.1. |
| **ACCEPTED-BOUNDARY** | Proof-to-code assumption accepted with bounded evidence (CI vectors, coverage tracking). Documented gap; acceptable for genesis lock with stated evidence. |
| **EXCLUDED** | Outside active v1.0 genesis-lock scope. The claim is not made for v1.0; no dependency on genesis lock. |
| **POST-GENESIS** | In scope for future migration but not active in v1.0 state-root path. |

---

## Axiom inventory

### AX-1: ISA correctness

| Field | Value |
|-------|-------|
| **Coq name** | Implicit (not named in a single file; assumed across all proofs) |
| **PDF anchor** | §3.11.1 — `AX-1 ISA correctness: [ASSUMED] authorized ISAs implement two's complement correctly` |
| **Class** | ACCEPTED-COMPILER |
| **Rationale** | Standard hardware correctness assumption for x86_64, aarch64, riscv64gc. Empirically verified through cross-ISA CI golden-root tests (TH-7). |

### AX-2: Rust compiler correctness (`AX2_rust_refinement`)

| Field | Value |
|-------|-------|
| **Coq name** | `AX2_rust_refinement` in `proofs/model/RefinementStatement.v` |
| **PDF anchor** | §3.11.1 — `AX-2 Compiler: [ASSUMED] pinned Rust toolchain produces correct code` |
| **Class** | ACCEPTED-BOUNDARY |
| **Rationale** | Proof-to-code observational equivalence is accepted with 12 CI test vectors (`coq_vectors.rs::coq_model_parity`, TV-0..TV-11). The axiom is non-vacuous: it is typed over specific observable model executions. AX2 is classified as an accepted AXIOM (compiler correctness assumption) in the Coq development; not an open gap or scheduled deliverable. |
| **Evidence** | `proofs/model/vectors.json`, `proofs/model/transition_observations.json`, `crates/consensus/tests/coq_vectors.rs`, `release-attestation.yml` |

### AX-3: SHA3-256 collision resistance (`AX3_sha3_assumed_injective`)

| Field | Value |
|-------|-------|
| **Coq name** | `AX3_sha3_assumed_injective` in `proofs/contractivity/encode_injectivity.v`; `sha3_256_collision_resistant` in `proofs/cascade/cascade_collision_resistance.v` |
| **PDF anchor** | §3.11.1 — `AX-3 Hash security: [ASSUMED] the active consensus hash suite (SHA3-256 + SM3-256, folded by SHA3-256) is modeled as collision-resistant over protocol state space` |
| **Class** | ACCEPTED-CRYPTOGRAPHIC |
| **Rationale** | Standard computational hardness assumption. SHA3-256 (FIPS 202) and SM3-256 (GM/T 0004) collision resistance is accepted for the v1.0 state-root commitment path. AX-3 is explicitly the active v1.0 genesis-lock gate (TH-1, TH-2, TH-8 all depend on it). |

### Cascade PRF security axioms

| Field | Value |
|-------|-------|
| **Coq names** | `cascade_prf_security`, `cascade_prf_quantitative_bound` in `proofs/blinding/blinding_non_interference.v` |
| **Class** | ACCEPTED-CRYPTOGRAPHIC (EXCLUDED from v1.0 active claim) |
| **Rationale** | PRF security of `H_cascade_keyed` is used for blinding key derivation in Domain B. Not a v1.0 Domain A state-root claim. Accepted typed axiom (non-vacuous `adv_le` bound). |

### GF(2¹²⁸) AU bound (`ghash_poly_mac_au_bound`)

| Field | Value |
|-------|-------|
| **Coq name** | `ghash_poly_mac_au_bound` in `proofs/cascade/it_mac_forgery_bound.v` |
| **Class** | ACCEPTED-CRYPTOGRAPHIC (EXCLUDED from v1.0 active claim) |
| **Rationale** | The IT-MAC forgery bound is used in the cascade derive path, a Domain B / Phase 2 feature. Not an active v1.0 Domain A claim. Accepted typed axiom (non-vacuous `adv_le` bound). |

### Cascade collision reduction (`cascade_collision_implies_sha3_collision`)

| Field | Value |
|-------|-------|
| **Coq name** | `cascade_collision_implies_sha3_collision` in `proofs/cascade/cascade_collision_resistance.v` |
| **Class** | POST-GENESIS |
| **Rationale** | Cascade-backed commitments and ZK proofs are a post-genesis migration item (TH-10). The active v1.0 state-root commitment uses SHA3-256 + SM3-256 (folded) — not the cascade. The `PLACEHOLDER` annotation in the Coq file is a formalization note for the reduction argument; it does not block v1.0. |

### Causal fingerprint hash axioms

| Field | Value |
|-------|-------|
| **Coq names** | `fp_hash_injective`, `fp_chain_collision_resistant` in `proofs/safety/causal_fingerprint.v` |
| **Class** | ACCEPTED-CRYPTOGRAPHIC |
| **Rationale** | These are SHA3-256 collision/preimage resistance assumptions named in context of the causal fingerprint proofs. They reduce to AX-3 by construction. |

---

## Placeholder inventory

### `privacy/blinding_health_metric.v`

| Field | Value |
|-------|-------|
| **Coq names** | `blinding_health_bounded`, `blinding_halt_monotone` (placeholder axioms) |
| **Class** | EXCLUDED |
| **Rationale** | `blinding_health` Lyapunov factor is not implemented in `lyapunov.rs`. Domain B blinding spec (§P8) is not a v1.0 active feature. No genesis-lock dependency. |

### `privacy/oblivious_access_non_interference.v`

| Field | Value |
|-------|-------|
| **Coq name** | `oram_access_non_interference` (placeholder axiom) |
| **Class** | EXCLUDED |
| **Rationale** | Deferred to Domain B blinding spec revision and `blinding_params` definition. Not a v1.0 active claim. |

### `privacy/receipt_proof_soundness.v`

| Field | Value |
|-------|-------|
| **Coq name** | `receipt_proof_soundness` (placeholder axiom) |
| **Class** | EXCLUDED |
| **Rationale** | Deferred to receipt spec (`06_receipts.md`) and Plonky3 FRI-STARK integration. Not a v1.0 active claim. |

### `privacy/cascade_avalanche_property.v`

| Field | Value |
|-------|-------|
| **Coq name** | Reclassified as STATISTICAL — not a formal proof target |
| **Class** | EXCLUDED |
| **Rationale** | Avalanche is an empirical/statistical property. Genesis security rests on collision/preimage resistance (AX-3), not avalanche. Reclassified from formal proof target to statistical/KAT evidence. |

---

## Unclassified axioms: zero

All axioms and placeholders in the proof corpus are classified above. No
unclassified axiom remains. No active PLACEHOLDER exists within v1.0 active claims.

---

## Acceptance statement

The v1.0 genesis-candidate proceeds with the following accepted axioms:

1. **AX-1** (ISA correctness) — empirically evidenced by cross-ISA CI
2. **AX-2** (Rust compiler) — accepted with 12 CI vector witnesses (TV-0..TV-11); AX2 is a Coq AXIOM (compiler correctness assumption), not an open gap
3. **AX-3** (SHA3-256 + SM3-256 collision resistance) — standard cryptographic assumption

All other axioms are either excluded from v1.0 active claims (Domain B features,
post-genesis items) or are typed non-vacuous computational assumptions within
their respective proof scopes.

This classification is consistent with `proofs/COVERAGE.md` proof-debt
classification and PDF §3.11 theorem dependency graph.
