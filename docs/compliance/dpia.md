# Data Protection Impact Assessment — QASH

**Document type:** GDPR Art. 35 DPIA (design-phase record)  
**Date:** 2026-05-26  
**Status:** Design-phase record — not a legal opinion or compliance certification  
**Scope:** QASH PAL (Domain B) receipt handling, key-shredding engine, and public transcript surface

> **Claim boundary:** This document records design-phase evidence for GDPR Art. 35
> assessment. It does NOT constitute legal advice or a compliance certification.
> Claim: "GDPR-aligned design with erasure-compatible receipt handling."
> Do NOT claim "GDPR compliant" — formal compliance requires legal assessment,
> supervisory authority consultation (if required), and DPO sign-off.

---

## 1. Processing Overview

QASH is an offline incident-log commitment and cyber-resilience evidence substrate.
It does not process personally identifiable information (PII) in Domain A
(deterministic consensus). Domain B (PAL) handles encrypted receipt material
on behalf of operators. The architecture is designed for data minimisation
and erasure-compatible handling of receipt data.

### 1.1 Data Categories

| Category | Location | PII Risk | Mitigation |
|----------|----------|----------|------------|
| State roots | `EpochState.state_root` (Domain A) | None — SHA3-256 hash of aggregated state | No PII in input to state root computation |
| Receipt roots | `EpochState.receipt_root` (Domain A) | None — commitment to encrypted receipt keys | Only key commitments cross the Domain A/B boundary |
| Encrypted receipts | Domain B WAL only | Potential — operator-defined content | Encryption at rest; key-shredding engine supports Art. 17 erasure |
| Public transcripts | `PublicTranscript` struct | None by construction | Compile-time field assertion (`assert_fields!`) |
| Validator IDs | `validator_ids: [[u8; 48]]` | Potential if IDs are public keys | No linkage to natural persons required by protocol |
| Log pseudonyms | `log_pseudonym()` output | Minimal — truncated SHA3-256 of public key | Pre-image resistance prevents key recovery; no IP/name logging |

### 1.2 Data Flows

```
Operator submits encrypted receipt
  → Domain B PAL: encrypted at rest with ReceiptKey (ZeroizeOnDrop)
  → ReceiptKey.key_commitment committed to receipt_root (Domain A)
  → Domain A: only key_commitment (SHA3-256) enters consensus state — no raw key bytes
  → On erasure request: shred_key() consumes ReceiptKey, fires ZeroizeOnDrop
  → ShredKeyEvidence archived to WAL (key_commitment + epoch + event_root)
  → Decryption becomes computationally infeasible (pre-image resistance)
```

---

## 2. Necessity and Proportionality (Art. 35(7)(b))

The protocol collects the minimum data necessary for its stated purpose
(offline incident-log commitment and cyber-resilience evidence):

- **State roots** — required for consensus correctness; contain no PII
- **Receipt roots** — required for commitment integrity; only SHA3-256 commitments, no raw data
- **Encrypted receipts** — operator-controlled; encryption ensures confidentiality; shredding supports erasure
- **Validator IDs** — required for consensus participation; not required to map to natural persons

No profiling, behavioral analysis, or automated decision-making occurs in the protocol.

---

## 3. Risks and Mitigations

| Risk | Likelihood | Severity | Mitigation | Residual Risk |
|------|-----------|----------|------------|---------------|
| Encrypted receipt content links to natural person | Medium | High | Encryption at rest; key-shredding on erasure request | Low (post-shred) |
| Validator ID links to natural person | Low | Medium | Protocol does not require natural-person identity for validators | Low |
| Log lines contain raw public keys or IP addresses | Low | Medium | `log_pseudonym()` replaces raw keys in logs; IP logging policy is operator responsibility | Low (operator residual) |
| State root pre-image reveals PII | Very Low | Low | SHA3-256 pre-image resistance; Domain A inputs contain no PII by design | Negligible |
| Cross-border transfer of encrypted receipts | Operator-dependent | Medium | Operator responsible for transfer impact assessment; protocol is jurisdiction-neutral | Operator residual |

---

## 4. Erasure-Compatible Design (Art. 17)

### 4.1 Key-Shredding Engine

`crates/pal/src/privacy/erasure.rs` implements erasure-compatible receipt handling:

- `ReceiptKey` — derives `ZeroizeOnDrop`; key material is cryptographically wiped on drop
- `shred_key(key, epoch, event_root)` — consumes the key by value; ZeroizeOnDrop fires; returns `ShredKeyEvidence`
- After `shred_key()` returns, decryption of the associated ciphertext is computationally infeasible
- `ShredKeyEvidence` provides an audit record: `key_commitment`, `epoch`, `event_root`

### 4.2 Scope and Limitations

Key shredding makes decryption computationally infeasible. It is one component
of a broader erasure-handling design. Full Art. 17 compliance also requires:

- Erasure request intake and tracking (operator responsibility)
- Confirmation that no backup copies of the plaintext key material exist
- Legal assessment of whether cryptographic erasure satisfies Art. 17 in the applicable jurisdiction
- DPO review if the controller is subject to Art. 37–39

This document records that the **implementation layer** supports erasure-compatible
handling. It does not assert that the full legal and organisational requirements
for Art. 17 compliance are met.

---

## 5. PublicTranscript PII Surface Audit

`crates/pal/src/privacy/public_transcript.rs` uses `static_assertions::assert_fields!`
to enforce at compile time that `PublicTranscript` contains only the five fields:
`state_root`, `receipt_root`, `efb_root`, `epoch`, `halt_flag`.

All five fields are SHA3-256 hash outputs or scalar protocol counters. None contains
PII. Adding a PII-bearing field to `PublicTranscript` would require updating the
`assert_fields!` call — providing a compile-time tripwire against accidental exposure.

---

## 6. Data Minimisation (Art. 5(1)(c))

The following design choices enforce data minimisation:

- **Domain A state struct contains no PII** — enforced by Domain A tripwire CI job (`check_domain_a_tripwires.sh`)
- **Only commitments cross the Domain A/B boundary** — CapToken schema proof (`proofs/capability/cap_token_schema.v`) formalises that crossing is explicit
- **Log pseudonyms** — `log_pseudonym()` in `crates/pal/src/crypto/tls.rs` outputs a 16-byte truncated SHA3-256 of the public key; raw keys and IP addresses are never logged by the protocol layer
- **Receipt encryption** — encrypted at rest with ephemeral per-receipt keys; only key commitments enter the consensus state

---

## 7. Consultation and Sign-Off

| Role | Action Required | Status |
|------|----------------|--------|
| Data Protection Officer | Review and sign off on DPIA before any production deployment | Pending (pre-genesis) |
| Legal counsel | Assess whether cryptographic erasure satisfies Art. 17 in target jurisdictions | Pending |
| Supervisory Authority | Consult under Art. 36 if residual high risk cannot be mitigated | Pending assessment |

This DPIA is a design-phase record. It must be reviewed and updated before
any deployment that processes personal data of EU data subjects.
