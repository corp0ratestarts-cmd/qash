# Pure QASH Privacy Model

**Status:** Normative  
**Scope:** Domain A public surface boundary and observer-class definitions for Pure QASH Core.

This is the Pure QASH privacy model. It contains Classes I–III only.
Class IV (regulatory authority / genesis-authorised disclosure) is NOT present in Pure QASH.
See `docs/spec/19_profile_taxonomy.md` in the umbrella repo for the full profile comparison.

---

## §P0 — Privacy posture

Pure QASH is a *graph-non-publishing* system, not a *graph-obfuscating* system.
External passive observers never receive a semantically meaningful transaction graph.

The public surface is a deterministic commitment synchronisation mesh, not a
transaction gossip network. The public transcript is root-only:
`(state_root, receipt_root, efb_root, epoch, halt_flag)`.

---

## §P0a — What Pure QASH does NOT protect against (normative)

| Threat | Why out of scope |
|--------|-----------------|
| Endpoint compromise | Cryptographic privacy holds only if keys are not extracted. |
| Physical coercion | Outside cryptographic scope. |
| Exchange/egress correlation | Economic layer; not consensus. |
| Global passive adversary (full network view) | Privacy is architectural (no graph broadcast), not transport-layer anonymity. |

---

## §P1 — Dual-layer network architecture

| Layer | Visible Artifacts |
|-------|-------------------|
| Domain B — Admission/Transport | Raw TX envelopes, clone chunks — never publicly broadcast |
| Domain A — Consensus/Sync | `state_root`, `receipt_root`, `efb_root`, epoch, halt_flag only |

---

## §P2 — Observer classes (Pure QASH — Classes I–III only)

### Class I — Public observer

> Can see: `(epoch, state_root, receipt_root, efb_root, halt_flag)`.  
> MUST NOT see: validator identities, transaction amounts, sender/receiver,
> envelope payloads, graph topology, receipt leaf values.

### Class II — Authorized validator

> Can see: own slot assignment, aggregated divergence metrics, blinded opcodes in own shard.  
> MUST NOT see: other validators' private keys, other TX plaintext, exact divergence
> decomposition of other validators.

### Class III — Receipt holder (own disclosure only)

> Can see: own receipts when holding the epoch viewing key derived from `epoch_seed`.  
> MUST NOT see: other receipts, graph adjacency, post-rotation receipts (forward secrecy).

### No Class IV in Pure QASH

Pure QASH has no Class IV (regulatory authority) observer class, no genesis-authorised
disclosure key, and no lawful-basis disclosure flows. These belong exclusively to the
QASH Regulated Profile (umbrella repo).

Any PR introducing Class IV, `disclosure_key`, `lawful_basis`, or `regulated_disclosure`
into this repo MUST be blocked by absence guards.

---

## §P3 — Public observation transcript

```
PublicTranscript(epoch_t) = {
  state_root_t,
  receipt_root_t,
  efb_root_t,
  epoch_t,
  halt_flag_t
}
```

The Rust type `qash_consensus::PublicTranscript` encodes this boundary at compile time.

---

## §P4 — Theorem targets

```
TH-P1 (Public graph non-observability):   [REQUIRED gate before genesis-candidate]
  For any two admissible transaction sequences T_a, T_b yielding identical
  epoch count and halt status, PublicTranscript(T_a) and PublicTranscript(T_b)
  are computationally indistinguishable under CPA. No semantic transaction graph,
  adjacency structure, or economic metadata is externally observable.

TH-P2 (Receipt non-disclosure):           [REQUIRED gate before genesis-candidate]
  receipt_root commits to blinded execution traces via the hash cascade.
  Without an epoch viewing key, no party can extract sender, receiver, amount,
  action type, or graph adjacency from receipt data or membership proofs.

TH-P3 (No user graph persistence in Pure QASH):   STATUS: TARGET
TH-P4 (Blind certification evidence non-disclosure): STATUS: TARGET
TH-P5 (Regulated profile absence in Pure QASH):     STATUS: TARGET
```

TH-P1 and TH-P2 are required evidence gates before genesis-candidate status.
A genesis-candidate PR missing these is inadmissible.
