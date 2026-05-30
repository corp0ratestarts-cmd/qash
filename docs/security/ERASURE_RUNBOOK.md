# Erasure Request Operator Runbook

**Status**: Implementation complete / self-tested  
**Date**: 2026-05-30  
**Scope**: QASH Domain B key-shredding erasure lifecycle (Art. 17 GDPR alignment)

> This runbook covers the technical operator steps for processing an erasure request.
> It does not constitute legal advice. GDPR compliance requires additional legal
> assessment, DPO review, and organisational procedures not covered here.

---

## Overview

QASH implements erasure-compatible receipt handling via key-shredding:

1. Receipts are encrypted with a per-receipt `ReceiptKey`.
2. The key is stored in the key store; only its SHA3-256 commitment is logged.
3. On erasure: the key is consumed by `process_erasure_request`, which zeroizes
   the key material and returns `ShredKeyEvidence`.
4. After shredding, decryption of the receipt is computationally infeasible
   (preimage resistance of SHA3-256 prevents recovering key from commitment).

---

## Prerequisites

- Access to the QASH Domain B key store (deployment-specific location).
- Ability to run `qash-pal` tooling or direct API access.
- The `receipt_commitment` (SHA3-256 of the key material) for the receipt to be erased.
- The protocol epoch at time of request.

---

## Step 1 — Receive the erasure request

Collect from the requestor:
- **Receipt commitment**: `[u8; 32]` — the `key_commitment` field logged when the receipt was created.
- **Requestor ID**: an opaque 32-byte identifier for audit tracing (do not log as plaintext PII).
- **Epoch**: current protocol epoch.

Construct:
```rust
ErasureRequest {
    receipt_commitment: <32-byte commitment>,
    requestor_id: <32-byte opaque id>,
    epoch: <current epoch>,
}
```

---

## Step 2 — Locate the key

Call:
```rust
let result = process_erasure_request(req, &mut key_store);
```

Internal state transitions: `PendingLocate → Located → Shredding → ShredComplete`.

Possible outcomes:

| Result | Meaning | Action |
|--------|---------|--------|
| `Ok(ShredKeyEvidence)` | Key found and shredded | Proceed to Step 3 |
| `Err(KeyNotFound)` | Key not in store (already shredded or never created) | Confirm with audit log; if already shredded, evidence exists |
| `Err(CommitmentMismatch)` | Store integrity error | Escalate to security team immediately |

---

## Step 3 — Persist the evidence

Before responding to the requestor, persist the `ShredKeyEvidence` to the WAL:

```rust
ShredKeyEvidence {
    key_commitment: <matches receipt_commitment>,
    epoch: <epoch from request>,
    event_root: <requestor_id>,
}
```

The evidence record must be:
- Written to append-only audit storage before the response is sent.
- Never deleted (it is the proof that the erasure occurred).
- Linked to the original request via `key_commitment` + `epoch`.

---

## Step 4 — Confirm erasure

Verify:
1. `evidence.key_commitment == req.receipt_commitment` — key identity confirmed.
2. The key store no longer contains an entry for this commitment.
3. A second call to `process_erasure_request` with the same request returns `Err(KeyNotFound)`.

---

## Step 5 — Respond to the requestor

After evidence is persisted:
- Notify the requestor that the erasure is complete.
- Provide the `evidence.epoch` as a timestamp reference.
- Do NOT provide the `ShredKeyEvidence` bytes directly to the requestor — this is an internal audit record.

---

## Backup and key-copy policy

- **No raw key backups**: `ReceiptKey.material` must never be copied to backup storage.
- **Encrypted-receipt backups**: The encrypted receipt (ciphertext) may be backed up; it is useless without the key.
- **Key store backups**: If the key store is backed up, the backup must be subject to the same erasure procedure. Erasure is complete only when the key is shredded from ALL copies.
- **Shredded keys cannot be restored**: Once `shred_key()` returns, the key material is gone. There is no recovery path. Operators must confirm this is intentional before proceeding.

---

## Non-claims

- This runbook does not constitute a GDPR compliance certification.
- Art. 17 GDPR compliance requires additional legal assessment, DPO oversight,
  data subject verification, response time tracking, and organisational controls
  not provided by this implementation.
- "GDPR-aligned design with erasure-compatible receipt handling" is the correct
  claim boundary — not "GDPR compliant."
