# QASH Protocol — Data Protection Impact Assessment (DPIA) Draft

**Regulation:** GDPR Article 35 (Data Protection Impact Assessment)  
**Status:** Draft — for review before any EU operator deployment.  
**Date:** 2026-05-24

---

## 1. Description of Processing

QASH is a deterministic consensus protocol. The protocol itself (Domain A)
processes only cryptographic commitments — it never sees plaintext transaction
data. Domain B (PAL) may process receipts, but these are encrypted and
subject to the disclosure domain controls described below.

### 1.1 Data Categories Processed

| Data Category | Location | GDPR Personal Data? | Notes |
|---------------|----------|---------------------|-------|
| State roots (SHA3-256 hashes) | `EpochState.state_root` | No | Cryptographic commitments only |
| Causal fingerprints | `EpochState.causal_fingerprint` | No | H(epoch history) — no personal data |
| Validator IDs (48 bytes) | `EpochState.validator_ids` | Potentially | If ID links to a natural person |
| Receipt ciphertexts | `ReceiptVault` (Domain B) | Potentially | Encrypted; plaintext never in WAL |
| Transaction nonces | `EpochState.nonces` | No | Monotone counter per validator slot |
| Entropy seed | `EpochState.entropy_seed` | No | Derived from CSPRNG, not user data |

### 1.2 Processing Purpose and Legal Basis

Purpose: deterministic consensus for digital cash transfers.  
Legal basis candidates: contract (Art. 6(1)(b)) or legitimate interest (Art. 6(1)(f)).  
Special categories (Art. 9): not applicable — no health, biometric, or political data.

---

## 2. Necessity and Proportionality

The QASH protocol is designed to minimize data retention:

- **Zero-persistence PAL**: only state roots and commitment evidence are
  persisted to the recovery WAL. No raw transaction bodies, peer identities,
  or graph-shaped metadata are written to durable storage.
- **Receipt encryption**: `ReceiptEncryptionProfile` encrypts receipt bodies.
  The public transcript contains only the ciphertext root.
- **Disclosure domain enforcement**: `DisclosureDomain::may_disclose_to()`
  never permits `Observer::PublicNetwork`. `HolderOnly` receipts may not be
  disclosed even to auditors without holder consent.
- **Key shredding**: `ShredCommitment` provides cryptographic evidence of
  plaintext destruction. This supports Art. 17 (right to erasure) by making
  it verifiable that the key material for a specific receipt has been destroyed.

---

## 3. Risks Identified

### R1 — Validator ID linkability

**Risk:** 48-byte validator IDs in the public state root commitment could link
a consensus participant to a natural person if the ID derivation scheme is
known.

**Mitigation:** 
- Validator IDs are fixed at genesis and not published in plaintext (only
  committed to the state root hash).
- Domain B key management must not expose raw validator IDs on public APIs.
- Operators must evaluate whether their ID derivation scheme creates linkage.

### R2 — Receipt metadata leakage

**Risk:** Even with encrypted receipt bodies, metadata patterns (timing,
frequency of commits by slot) could enable inference attacks.

**Mitigation:**
- `DisclosureDomain::HolderOnly` prevents auditor or operator disclosure
  without holder consent.
- Domain B may implement cover traffic (see `clone_protocol` spec note).
- No receipt frequency metadata is written to the recovery WAL.

### R3 — Cross-shard receipt aggregation

**Risk:** `M2` audit finding — cross-shard receipts excluded from state root
(`include_in_state_root = false`). If receipt roots are later included, the
state root may commit to personal data.

**Mitigation:** The current v1.2 sharding implementation includes
`aggregate_receipt_root` which commits to encrypted receipt roots only —
no plaintext data. A formal proof that exclusion preserves replay-determinism
is tracked in `proofs/sharding/`.

---

## 4. Measures to Address Risks

| Measure | Implementation | Evidence |
|---------|----------------|---------|
| Data minimization | Zero-persistence WAL; receipt encryption profile | `crates/pal/src/recovery_wal.rs`; `crates/pal/src/receipt.rs` |
| Right to erasure (Art. 17) | `ShredCommitment` + key shredding | `crates/pal/src/receipt.rs` |
| Purpose limitation | Disclosure domain enforcement gate | `DisclosureDomain::may_disclose_to()` |
| Confidentiality | AES-256-GCM or ML-KEM-768-WRAP receipt encryption | `ReceiptEncryptionProfile` |
| Data subject access | Holder-controlled disclosure via `HolderOnly` domain | `crates/pal/src/receipt.rs` |
| Privacy by design | Domain A processes only commitments; no personal data enters consensus | `docs/spec/00_execution_model.md` §3 |

---

## 5. Residual Risks and Open Items

| Item | Severity | Stage |
|------|----------|-------|
| M2: Cross-shard receipt exclusion proof | Medium | Stage 7 / proofs/sharding/ |
| M3: tx\_id over authenticated fields only | Medium | Stage 4 (post PQC signature) |
| Validator ID linkability analysis | Low | Pre-genesis operator guidance |
| DPA consultation requirement | Per-deployment | Stage 9 / deployment checklist |

---

## 6. DPA Prior Consultation (Art. 36)

Prior consultation with a Data Protection Authority (DPA) is recommended if:
- The processing is systematic and large-scale (many validators or high tx volume)
- The validator IDs are derived from directly-identifying information
- The deployment processes sensitive categories of data

QASH protocol authors are not data controllers for any specific deployment.
Each operator must conduct their own Article 35 DPIA and consult their DPA
if required.

---

## 7. Review Schedule

This DPIA should be reviewed:
- When the genesis constants are locked (Stage 9)
- When the production PQC verifier is wired (Stage 4)
- When cross-shard receipt roots are included in state root (M2 resolution)
- At least annually after deployment
