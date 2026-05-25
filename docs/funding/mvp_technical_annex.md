# QASH MVP — Technical Annex

**For use in:** Innovate UK, EU Horizon, SBRI, and equivalent funding applications  
**Framing:** Cyber-resilience substrate for deterministic replay and selective disclosure of offline incident-log commitments without publishing a transaction graph.

---

## Executive Summary

QASH demonstrates a local, offline-capable incident-log attestation substrate. Operators can commit incident records as cryptographic commitments to a local append-only write-ahead log (WAL), export a commitment-only public transcript, replay that transcript deterministically across machines, and selectively disclose individual incident bodies to authorised parties — without exposing a transaction graph or private payload data.

The system is not a payment rail, token, or financial instrument. It is a cryptographic substrate for audit-trail integrity and selective disclosure in cyber-resilience and operational contexts.

---

## Technical Description

### Core construct: `TX-MVP-ReceiptCommit`

Each incident record produces a 140-byte fixed-size commitment transaction containing:

| Field | Size | Description |
|-------|------|-------------|
| `version` | 4 bytes | Protocol version tag |
| `epoch` | 8 bytes | Logical epoch of the incident |
| `nonce` | 32 bytes | Per-record nonce (workspace-salt derived) |
| `payload_commitment` | 32 bytes | SHA3-256 commitment to the incident body |
| `disclosure_key_commitment` | 32 bytes | Commitment to a selective-disclosure key |
| `domain_tag` | 32 bytes | Fixed domain separator |

The raw nonce and incident body never leave the local vault. The public export contains commitments only: `tx_commitment`, `nonce_commitment`, `payload_commitment`, `disclosure_key_commitment`.

### Local vault and WAL

The vault (`MvpReceiptVault`) maintains an append-only WAL (`commitments.wal`) storing both the private full record (for local disclosure) and the derived public export. The WAL uses fixed-size records with magic headers, allowing detection of truncation and corruption.

A workspace salt (`vault_salt.bin`) is generated at initialisation and used to derive per-record nonces deterministically, preventing replay collisions without requiring external randomness at record-issue time.

### Deterministic replay

The `replay` command folds public commitment exports through a domain-tagged SHA3-256 accumulator:

```
root_n = SHA3-256("QASH-MVP-REPLAY-ROOT\0" || root_{n-1} || len(record) || record)
```

This produces a stable `commitment_root` that is identical across machines and runs given the same sequence of public records, enabling independent verification of the audit trail without exposing private content.

### Selective disclosure

The `disclose` command exports a single incident body — authenticated by receipt ID — alongside its commitment. The exported bundle does not contain any other incident body. Disclosure is impossible for records held only as imported public commitments (the private body is required).

### Import-side sync

A second vault (e.g., a peer audit node) can import a `public_commitments.bin` export from any originating vault. The peer can replay to verify the same `commitment_root` but cannot disclose receipts it does not hold privately. This enables distributed verification without gossip of private incident data.

---

## Innovation

1. **No transaction graph exposure.** Standard audit systems log structured events that can be correlated. QASH exports only cryptographic commitments, making it impossible to recover the incident body or timing from the public transcript alone.

2. **Offline-first design.** The full attestation flow — init, issue-receipt, sync, replay, disclose — requires no network connectivity. The system is designed for air-gapped and intermittently-connected operational environments.

3. **Deterministic replay invariant.** The replay root is identical across ISA architectures (x86-64, AArch64, RISC-V) given the same input, enabling cross-platform audit verification without trusted intermediaries.

4. **Selective disclosure without ZK proofs.** Selective disclosure is achieved through commitment-based revelation rather than zero-knowledge proofs, making the scheme auditable with standard tooling while preserving privacy for undisclosed records.

---

## Current Capabilities (TRL 3–4)

| Capability | Status |
|------------|--------|
| Local incident receipt commitment | ✅ Implemented and tested |
| Commitment-only public export | ✅ Implemented and tested |
| Deterministic local replay | ✅ Verified across two runs |
| One-receipt selective disclosure | ✅ Implemented and tested |
| Import-side replay (peer node) | ✅ Implemented and tested |
| Workspace-salt nonce derivation | ✅ Implemented |
| WAL corruption detection | ✅ Tested (truncation, bad magic, duplicate records) |
| Offline-first operation | ✅ No network dependency |
| Multi-party networked consensus | ❌ Not in scope for MVP |
| Production hardware attestation | ❌ Not in scope for MVP |
| ZK proof integration | ❌ Not in scope for MVP |

---

## Claim Boundary

All claims in this annex are limited to the MVP demonstrator scope defined in `docs/mvp/claims_register.md`. This system is not a payment instrument, settlement rail, regulated financial product, or production deployment.
