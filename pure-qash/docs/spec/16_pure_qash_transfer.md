# PTX-0 — Pure QASH Cash Transfer

**Status:** Normative spec target (not yet implemented)  
**Scope:** MEV-null cash transfer transaction type for Pure QASH.

PTX-0 is transaction type 0 in the Pure QASH registry. It is unrelated to
umbrella QASH TX-0 (NoOp consensus heartbeat). The "P" prefix distinguishes
Pure QASH transaction types from umbrella types.

---

## §16.1 — Design principles

PTX-0 uses note/nullifier semantics rather than account-order transfer semantics.
This avoids first-wins ordering dependence and enables commutativity for
non-conflicting transfers.

Key properties:
- No public amount
- No public sender
- No public receiver
- No stable user identifier
- No memo or padding field usable for grinding
- No application callback
- No oracle reference
- No priority fee

---

## §16.2 — Domain B admission boundary

**Critical:** PTX-0 involves ZK proofs (`value_balance_proof`, `authorization_proof`).
These proofs are verified in Domain B (PAL), NOT in Domain A (consensus).

Domain B receives raw PTX-0 bytes and:
1. Verifies `value_balance_proof` (Plonky3 backend)
2. Verifies `authorization_proof` (Plonky3 backend)
3. Produces a `CapToken<ValidatedEffect>` containing only commitment fields

Domain A receives only the `CapToken<ValidatedEffect>`. Raw proof bytes and witness data
never enter Domain A. Domain A performs no ZK verification.

This preserves the `no_std`, no-alloc, no-Plonky3 constraint on Domain A.

---

## §16.3 — What Domain A receives

```
// Domain A ValidatedEffect for PTX-0
pub struct Ptx0Effect {
    pub validated_nullifiers_root:  [u8; 32],  // commitment over spent nullifiers
    pub output_commitments_root:    [u8; 32],  // commitment over new output notes
    pub fee_amount:                 u128,       // exact required_fee, verified by Domain B
    pub effect_root:                [u8; 32],  // commitment over the full validated effect
}
```

---

## §16.4 — Validity rule (Domain A)

```
all input nullifiers are unspent at epoch start
fee_amount == required_fee(PTX0_TYPE, 0)    // payload_len = 0 for pure transfer
output commitments are well-formed
effect_root commits to all above fields
```

---

## §16.5 — Conflict rule (annihilation, not FIFO)

```
If two or more PTX-0 transactions in the same epoch share any input nullifier,
ALL transactions in that conflict class are rejected before application.
```

This avoids order-dependent first-wins behavior. Non-conflicting transfers commute.

---

## §16.6 — OrderImage for PTX-0

```
OrderImage(ptx0) =
  tx_type_le2 || validated_nullifiers_root || output_commitments_root
  || fee_amount_le16 || epoch_domain_le8
```

No `nonce`, no `author_id` — nullifier-based transfers use neither.

The `sort_key` uses `OrderImage` only. Proof bytes and authorization data are
excluded from the ordering image to prevent signature/proof grinding.

---

## §16.7 — Privacy invariants

- [ ] No public amount in any wire field
- [ ] No public sender or receiver
- [ ] No stable user identifier
- [ ] No memo or padding field usable for grinding
- [ ] No app callback, no oracle reference
- [ ] Non-conflicting transfers commute
- [ ] Double-spend conflicts annihilate (not FIFO)

---

## §16.8 — Admission checklist (pre-genesis)

Per `docs/spec/03_transactions.md §A8` admission requirements:

```yaml
epoch_unlinkable_identity_scheme:
  type: "nullifier-based; no stable user identifier"
  linkage_surface: "none"
  unlinkability_argument: "nullifiers are one-time; no sender/receiver in public fields"
  coq_proof_obligation: "proofs/privacy/ptx0_unlinkability.v (STATUS: TARGET)"

receipt_privacy:
  body_encryption: "ChaCha20-Poly1305; key derived from epoch_seed via cascade"
  disclosure_domain: "none (Pure QASH has no disclosure domain)"
  plaintext_at_halt: false
```
