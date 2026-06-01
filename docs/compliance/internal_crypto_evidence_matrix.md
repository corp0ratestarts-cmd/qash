# QASH Internal Cryptographic Evidence Matrix

Internal evidence record. Not a certification claim. See `docs/security/SECURITY_POLICY.md`
for the explicit non-claims section.

Last updated: 2026-05-30. Reflects genesis status `provisional`.

## Legend

| Field | Meaning |
|-------|---------|
| **Domain** | A = deterministic consensus (`crates/consensus/`), B = PAL/operational (`crates/pal/`, `src/`) |
| **Feature flag** | Cargo feature required to enable; `default` = always compiled |
| **KAT status** | `pass` = known-answer test in CI; `none` = no KAT yet; `advisory` = KAT present but not blocking |
| **Claim boundary** | `implemented` = code present and tested; `aligned` = design aligns, not fully tested; `scaffold` = stub only; `not-claimed` = explicitly excluded from v1.0 claims |

---

## Hash Primitives

| Algorithm | Module / Path | Domain | Feature flag | KAT status | Dependency source | Claim boundary | Evidence artifact |
|-----------|---------------|--------|--------------|------------|-------------------|----------------|-------------------|
| SHA3-256 | `crates/consensus/src/hash.rs` | A | default | pass | `sha3 v0.10.8` | implemented | `cavp-kat` CI job; NIST FIPS 202 vectors |
| SHA3-512 (cascade L1) | `crates/consensus/src/cascade.rs` | A | default | pass | `sha3 v0.10.8` | implemented | `tests/vectors/cascade_kat.json` |
| BLAKE3-XOF-64 (cascade L2) | `crates/consensus/src/cascade.rs` | A | default | pass | `blake3 v1.5.5 pure` | implemented | `tests/vectors/cascade_kat.json` |
| KangarooTwelve-XOF-64 (cascade L3) | `crates/consensus/src/cascade.rs` | A | default | pass | `tiny-keccak v2 k12` | implemented | `tests/vectors/cascade_kat.json` |
| SM3-double-width (cascade L4) | `crates/consensus/src/cascade.rs` | A | default | pass | `sm3 v0.4` | implemented | `tests/vectors/cascade_kat.json` |
| Streebog-512 (cascade L5) | `crates/consensus/src/cascade.rs` | A | default | pass | `streebog v0.10` | implemented | `tests/vectors/cascade_kat.json` |
| Kupyna-512 (cascade L6) | `crates/consensus/src/cascade.rs` | A | default | pass | `kupyna v0.1.0` | implemented | `tests/vectors/cascade_kat.json` |
| LSH-512 (cascade L7) | `crates/consensus/src/lsh512.rs` | A | default | pass | repo-local, pure ARX | implemented | `tests/vectors/cascade_kat.json` |
| SM3-256 (state root) | `crates/consensus/src/hash.rs` | A | default | pass | `sm3 v0.4` | implemented | `cavp-kat` CI job |
| H_domain (SHA3-256 domain-sep) | `crates/consensus/src/transition.rs` | A | default | pass | repo-local | implemented | `tests/vectors/vectors.v1.json` |
| H_cascade_keyed | `crates/consensus/src/cascade.rs` | B | default | pass | repo-local | implemented | `tests/vectors/cascade_kat.json` |

---

## Signature Schemes

| Algorithm | Module / Path | Domain | Feature flag | KAT status | Dependency source | Claim boundary | Evidence artifact |
|-----------|---------------|--------|--------------|------------|-------------------|----------------|-------------------|
| Dilithium5 (ML-DSA, primary) | `crates/pal/src/signing/` | B | default | advisory | `pqcrypto-dilithium` | implemented | Domain B unit tests |
| SLH-DSA-SHA3-256 (anchor) | `crates/pal/src/signing/` | B | default | advisory | `pqcrypto-sphincsplus` | implemented | Domain B unit tests |
| Falcon-512 (fallback) | `crates/pal/src/signing/` | B | default | advisory | `pqcrypto-falcon` | scaffold | Placeholder; not activated in v1.0 |

---

## Key Encapsulation / Transport

| Algorithm | Module / Path | Domain | Feature flag | KAT status | Dependency source | Claim boundary | Evidence artifact |
|-----------|---------------|--------|--------------|------------|-------------------|----------------|-------------------|
| X-Wing (KEM, transport) | `crates/pal/src/crypto/kem.rs` | B | default | pass | `x-wing` crate | implemented | `cavp-kat` CI job (ML-KEM-768) |
| ML-KEM-768 (KEM component) | `crates/pal/src/crypto/kem.rs` | B | `pqc` | pass | `ml-kem` | implemented | `cavp-kat` CI job; FIPS 203 vectors |

---

## DRBG / Key Derivation

| Algorithm | Module / Path | Domain | Feature flag | KAT status | Dependency source | Claim boundary | Evidence artifact |
|-----------|---------------|--------|--------------|------------|-------------------|----------------|-------------------|
| HMAC-SHA-256 DRBG | `crates/pal/src/crypto/drbg.rs` | B | default | pass | repo-local | implemented | `cavp-kat` CI job; RFC 4231 / SP 800-198 vectors |
| Blinding key derivation (`derive_epoch_blinding_key`) | `crates/pal/src/` | B | default | advisory | uses H_cascade_keyed | implemented | Domain B unit tests |

---

## ZK / Recursive Proofs

| Algorithm | Module / Path | Domain | Feature flag | KAT status | Dependency source | Claim boundary | Evidence artifact |
|-----------|---------------|--------|--------------|------------|-------------------|----------------|-------------------|
| Plonky3 FRI-STARK (two-layer) | `crates/pal/src/zk/backend.rs` | B | `plonky3,std` | pass | `p3-*` crates | implemented | `cavp-kat` CI job (corpus KAT) |

---

## Threshold / Multi-Party

| Algorithm | Module / Path | Domain | Feature flag | KAT status | Dependency source | Claim boundary | Evidence artifact |
|-----------|---------------|--------|--------------|------------|-------------------|----------------|-------------------|
| Threshold Dilithium5 (TALUS) | `crates/pal/src/threshold/talus.rs` | B | `threshold-signing` | none | TBD | scaffold | Not activated in v1.0 |

---

## IT-MAC / Information-Theoretic

| Algorithm | Module / Path | Domain | Feature flag | KAT status | Dependency source | Claim boundary | Evidence artifact |
|-----------|---------------|--------|--------------|------------|-------------------|----------------|-------------------|
| GF(2¹²⁸) IT-MAC | `proofs/cascade/it_mac_forgery_bound.v` | B | default (proof only) | none | repo-local Coq proof | not-claimed | Proof-adjacent. Forgery bound 16/2¹²⁸; Domain B Phase 2. Not an active v1.0 claim. |

---

## Internal Hedged Constructions

| Algorithm/construction | Module / Path | Domain | Feature flag | KAT status | Claim boundary | Evidence artifact |
|------------------------|---------------|--------|--------------|------------|----------------|-------------------|
| QASH dual_hash_32 / dual_hash_pair_32 | `crates/pal/src/crypto/dual_hash.rs` | B | none (always compiled in PAL) | internal unit tests | internal hedged construction; not certification evidence | `dual_hash_*` tests in `dual_hash.rs` |
| QASH QashHedgedDrbg | `crates/pal/src/crypto/hedged_drbg.rs` | B | `std` | internal unit tests | QASH-specific non-FIPS hedged DRBG; not SP 800-90A; not certification evidence | `hedged_drbg::tests::*` unit tests |

> **Claim boundary**: `dual_hash_32` and `dual_hash_pair_32` are QASH-specific internal Domain B constructions. They are not FIPS validated, not CAVP/ACVP evidence, and do not constitute a standards-conformant construction. They do not alter Domain A consensus, QASH-CASCADE-7, or GRC-7-7-v2.

---

## Explicit Non-Claims

The following algorithms/properties are explicitly **not claimed** for v1.0:

- **Cascade avalanche** (formal proof): Reclassified to statistical/KAT evidence. Genesis security rests on collision/preimage resistance (AX-3), not avalanche. See `proofs/privacy/cascade_avalanche_property.v`.
- **ORAM access non-interference**: Domain B blinding parameters not yet defined. Excluded from v1.0 active claim boundary. See `proofs/privacy/oblivious_access_non_interference.v`.
- **Blinding PRF security** (formal game): AU game proof deferred to post-genesis. Blinding key derivation is implemented and tested but not formally game-proved. See `proofs/blinding/blinding_non_interference.v`.
- **ZK proofs as a consensus-layer claim**: Plonky3 is a Domain B operational feature; its presence does not make ZK verification part of the consensus state-root definition.
- **Hardware attestation correctness**: TPM/TDX/SEV-SNP/ARM-CCA backends are scaffolds; correctness claims require platform-specific hardware access not available in the repo CI environment.
- **Threshold signing correctness**: TALUS is a scaffold; not activated or claimed in v1.0.
<!-- claim-boundary-allow: explicit non-claim disclaimer listing prohibited phrases to clarify scope -->
- **External certification / no FIPS validation**: This matrix records internal evidence only. No claim is made of FIPS validation, external certification, or regulatory approval. See `docs/security/SECURITY_POLICY.md`.
