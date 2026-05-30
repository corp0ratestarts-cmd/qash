# QASH CNSA 2.0 / PQC Profile

**Date:** 2026-05-30  
**Status:** Internal alignment (not externally validated)  
**Audience:** Security reviewers, compliance advisors, integration partners, auditors.

---

## Non-Claims Boundary

This document describes which post-quantum cryptographic algorithms are implemented,
scaffolded, or not yet claimed in QASH. It does **not** claim that QASH, QASH Labs,
or any QASH deployment is currently certified, approved, or validated under CNSA Suite 2.0
or any derivative programme.

"CNSA Suite 2.0 compliant" is **not claimed** — this designation requires formal NSA
approval following an evaluation process that has not been initiated.

The permitted claim is:

> QASH implements or scaffolds a subset of the algorithms enumerated in NSA's CNSA 2.0
> algorithm list, using FIPS 203 / FIPS 204 / FIPS 205 drafts as the implementation
> reference. This constitutes internal alignment, not CNSA 2.0 certification.

The prohibited claims are:

- "CNSA Suite 2.0 compliant"
- "NSA approved"
- "CNSA approved"
- any statement that implies formal NSA evaluation or endorsement.

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

## Profile Matrix

| Algorithm | Role | Feature flag | CNSA 2.0 use case | QASH status | KAT / evidence |
|-----------|------|-------------|-------------------|-------------|----------------|
| ML-KEM-768 | KEM | `pqc` | Key establishment | Implementation complete / self-tested | CI-verified via `cavp-kat` job; FIPS 203 vectors |
| Dilithium5 (ML-DSA) | Signature (primary) | advisory / planned | Authentication | Internal alignment (scaffold) | Domain B unit tests; full FIPS 204 alignment pending |
| SLH-DSA-SHA3-256 | Signature (cascade anchor) | advisory / planned | Authentication | Internal alignment (scaffold) | Domain B unit tests; full FIPS 205 alignment pending |
| SHA3-256 | Hash | default (always-on) | Hashing | Implementation complete / self-tested | CI-verified; Domain A state-root + Domain B key derivation |

### ML-KEM-768 detail

- Implementation: `crates/pal/src/crypto/kem.rs`, `ml-kem` crate, feature flag `pqc`
- Reference: FIPS 203 (Module-Lattice-Based Key-Encapsulation Mechanism)
- KAT: `cavp-kat` CI job; encap/decap round-trip test (`crypto::kem::tests::kat_encap_decap_roundtrip`)
- Domain: B (PAL / operational)

### Dilithium5 (ML-DSA) detail

- Implementation location: `crates/pal/src/signing/` — scaffold present, not production-deployed
- Reference target: FIPS 204 (Module-Lattice-Based Digital Signature Algorithm)
- Status note: Signature primitives are scaffolded; full ML-DSA FIPS 204 alignment is pending.
  **Signature CNSA compliance is not claimed.**

### SLH-DSA-SHA3-256 detail

- Implementation location: `crates/pal/src/signing/` — scaffold present, not production-deployed
- Role: cascade anchor signature
- Reference target: FIPS 205 (Stateless Hash-Based Digital Signature Standard)
- Status note: Scaffold only; full FIPS 205 alignment is pending.
  **Signature CNSA compliance is not claimed.**

### SHA3-256 detail

- Domain A usage: `crates/consensus/src/hash.rs` — state-root hashing (outside FIPS module boundary)
- Domain B usage: `crates/pal/src/crypto/` — key derivation inputs
- Reference: FIPS 202
- KAT: `cavp-kat` CI job; NIST FIPS 202 vectors (`hash::tests::sha3_256_known_vector`)

---

## Hybrid Mode: X-Wing Combiner

The X-Wing combiner (`xwing_combine` in `crates/pal/src/crypto/kem.rs`) combines
ML-KEM-768 and X25519 into a hybrid KEM for forward-secrecy transport.

| Property | Status |
|----------|--------|
| Implementation | Complete — `x-wing` crate, Domain B |
| External validation | Not externally validated |
| Test vectors | CI round-trip test present; official X-Wing draft test vectors not yet integrated |

---

## Evidence Gaps (Negative Tests)

The following negative test cases are identified as evidence gaps. They are required
before any claim of production-grade PQC key establishment can be made:

| Gap | Description | Priority |
|-----|-------------|----------|
| Wrong public key | Decapsulation attempt with mismatched public key — must return a decoy shared secret (IND-CCA2 behaviour), not an error that leaks timing information | High |
| Wrong ciphertext | Decapsulation of malformed or truncated ciphertext — must not panic or expose internal state | High |
| Wrong transcript binding | X-Wing combiner with modified session transcript binding — must produce a different and unusable shared secret | High |

These gaps do not imply that the current implementation is incorrect; they indicate
that the evidence base is incomplete for a production claim.

---

## Explicit Non-Claims

The following statements are explicitly **not** made for v1.0:

- **"CNSA Suite 2.0 compliant"** — this designation requires formal NSA approval following
  an evaluation process that has not been initiated.
- **Signature CNSA compliance** — Dilithium5 (ML-DSA) and SLH-DSA-SHA3-256 are scaffolded
  in Domain B (`crates/pal/src/signing/`) and are not production-deployed. CNSA 2.0
  signature compliance is not claimed.
- **Falcon-512 CNSA alignment** — Falcon-512 is a fallback scaffold; it is not listed
  in CNSA 2.0 and is not claimed.
- **External KAT validation** — CI KATs are self-authored. No NIST ACVP submission or
  third-party cryptographic lab validation has been completed.
- **Constant-time guarantees** — Domain B crypto paths have not been audited for
  constant-time behaviour using a formal tool (e.g. `dudect-bencher`). This is planned
  for Phase 2-P.

---

## Roadmap

| Phase | Action |
|-------|--------|
| 2-G (done) | ML-KEM-768 KAT in CI; X-Wing combiner implementation |
| 2-P | Add `dudect-bencher` constant-time audit for all Domain B PQC paths |
| 2-P | Integrate official X-Wing draft test vectors |
| 2-P | Add negative KATs: wrong public key, wrong ciphertext, wrong transcript binding |
| 2-G extension | Complete Dilithium5 / ML-DSA FIPS 204 alignment; promote from scaffold |
| 2-G extension | Complete SLH-DSA-SHA3-256 FIPS 205 alignment; promote from scaffold |
| Post-Genesis | Evaluate NIST ACVP submission for ML-KEM-768 and SHA3-256 |
