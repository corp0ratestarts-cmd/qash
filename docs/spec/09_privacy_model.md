# QASH Privacy Model

**Status:** Normative  
**Scope:** Domain A public surface boundary and observer-class definitions.
Domain B blinding implementation and formal proof obligations are deferred
(see §P8, §P10).

---

## §P0 — Privacy posture

QASH is a *graph-non-publishing* system, not a *graph-obfuscating* system.
External passive observers never receive a semantically meaningful transaction
graph.

The distinction from Monero/Zcash-style systems is architectural: those systems
broadcast a public graph and then cryptographically obscure its semantics; QASH
does not broadcast a graph at all.  The public surface is a deterministic
commitment synchronisation mesh, not a transaction gossip network.

---

## §P1 — Dual-layer network architecture

| Layer | Purpose | Visible Artifacts | Network Behaviour |
|-------|---------|-------------------|-------------------|
| **Domain B — Admission/Transport** | Private TX ingestion, offline sync, OEM/TEE execution | Raw TX envelopes, clone chunks, attestation quotes | QR/NFC/BLE, secure relays, TEE channels.  Never publicly broadcast. |
| **Domain A — Consensus/Sync** | State commitment propagation, Lyapunov verification, epoch synchronisation | `state_root`, `receipt_root`, Lyapunov proofs, validator attestations, epoch seeds | Structured P2P mesh for commitment-only gossip.  No TXs, no mempool, no edge graphs. |

---

## §P2 — Transaction lifecycle visibility boundaries

Each step of the transaction lifecycle (§Transaction lifecycle in
`03_transactions.md`) is explicitly layer-bound:

```
Step 1: Decoding    → Domain B (private). Bytes arrive via clone channels,
                      TEE-protected relays, or secure admission queues.
                      Never decoded on public network interfaces.
Step 2: Admission   → Domain B (private). Cryptographic blinding, nonce
                      validation, Lyapunov admission checks. Inside PAL/OEM
                      domains or validator TEEs.
Step 3: Ordering    → Domain B → Domain A boundary (internal). Deterministic
                      PRF-derived epoch schedule. No public ordering transcript.
Step 4: Application → Domain B blinded VM (opaque). Produces blinded execution
                      traces. Intermediates never externally visible.
Step 5: Replay      → Domain A (commitment-only). Validators verify state_root
                      and receipt_root continuity. Public observers see only
                      root evolution.
```

**Normative rule:** Raw transaction envelopes (including `author_id`, `nonce`,
`payload_len`, `payload`, `signature`) are Domain B artifacts.  They MUST NOT
appear in `PublicTranscript` (see §P3).

---

## §P3 — Public observation transcript

```
PublicTranscript(epoch_t) = {
  state_root_t,
  receipt_root_t,
  epoch_t,
  halt_flag_t
}
```

The Rust type `qash_consensus::PublicTranscript` (`crates/consensus/src/public.rs`)
encodes this boundary as a compile-time guarantee.  Any Domain A code path that
inadvertently includes raw envelope fields in the public surface will fail to
compile.

All artifacts in `PublicTranscript` are:

- Fixed-length (padded to protocol maximum).
- Uniform type tags (constant epoch commitment structure).
- Indistinguishable under adaptive observation (cascade avalanche ensures no
  semantic leakage for observers without disclosure keys).
- Decoupled from sender/receiver/amount/action type.

---

## §P4 — Observer classes

| Observer | Domain Access | Visible Artifacts | Graph/Edge Exposure |
|----------|---------------|-------------------|---------------------|
| Passive internet observer | Domain A only | `state_root`, `receipt_root`, Lyapunov proofs, epoch seeds | None.  Fixed-size, constant-rate emissions. |
| Light client (Tier 0–1) | Domain A | State-root chain continuity, sparse Merkle proofs against genesis cascade config | Zero transaction semantics.  Commitment validity only. |
| Full validator (Tier 2–4) | Domain A + B (TEE/OEM) | Blinded opcodes, cascade receipts, validator attestations, admission queue metadata | Cryptographically decoupled from real-world identities.  Execution traces are blinded and epoch-relative. |
| Receipt verifier / Auditor | Domain A + scoped Domain B | `receipt_root` + ZK membership proofs.  Decrypted receipts only with genesis-bound disclosure keys. | No adjacency reconstruction without explicit disclosure capability. |
| Clone peer / offline sync | Domain B (detached) | Encrypted clone chunks, Merkle proofs, sync receipts | Temporal isolation from public epoch transitions. |
| Compromised endpoint | Out of protocol scope | — | TEE/OEM layer concern; protocol halts rather than degrades. |

---

## §P5 — Validator identity vs user identity

`author_id: [u8; 48]` in TX-0 is a stable *validator* consensus identity.
Validators are a finite, known set of consensus participants.  Their identity
linkage across epochs is intentional and irreversible — validators are
accountable.  This is NOT a user privacy concern because QASH validators are
distinct from application-layer users.

TX-0 is a consensus heartbeat (nonce advance).  Its `author_id` carries no
sender→receiver→amount edge semantics.

Future user-facing transaction types (TX-2+) that carry user payment identities
MUST NOT use stable identifiers.  Their §A8 proof obligation MUST include an
**epoch-unlinkable identity declaration** with the following required fields:

```yaml
epoch_unlinkable_identity_scheme:
  type: <e.g. "cascade-derived ephemeral per-epoch key" | "ZK-pseudonym" | ...>
  linkage_surface: "none" | "within-epoch" | "within-disclosure-domain"
  unlinkability_argument: <proof sketch or reference>
  coq_proof_obligation: <proof file reference>
```

This requirement is enforceable at spec-review time (pre-genesis admission of the
TX-2+ spec document), not post-genesis.

**Pre-genesis admission checklist for TX-type specs:**

- [ ] §A8 proof obligation references a Coq file (even if `Admitted` temporarily)
- [ ] `epoch_unlinkable_identity_scheme` declares `linkage_surface = "none"` or
      `"within-disclosure-domain"`
- [ ] `receipt_privacy.body_encryption` specifies key derivation from cascade or
      epoch seed (see §P7)
- [ ] `receipt_privacy.plaintext_at_halt = false` is explicitly stated
- [ ] No stable user identifiers appear in public-facing fields

A TX-type spec that fails any checklist item is inadmissible.  No post-genesis
governance process may waive these requirements.

---

## §P6 — Leaf obfuscation

Sparse Merkle leaf positions are epoch-relative, derived from
`sort_key(entropy_seed_t, TxID(τ))` where `entropy_seed_t` changes every epoch
(`leaf_index_bytes = 48`, `sparse_merkle_depth = 384` from
`GENESIS_CONSTANTS.toml`).

A static Merkle position analysis cannot link transactions across epochs.

---

## §P7 — Receipt privacy

`receipt_root` commits to blinded execution traces via the hash cascade
(SHA3-256 → BLAKE3 → KangarooTwelve).  Without a genesis-authorised disclosure
capability, no party can extract sender, receiver, amount, action type, or graph
adjacency from receipt data or membership proofs.

Receipt bodies are encrypted by default.  Selective disclosure requires an
explicit disclosure key scoped to a genesis-authorised disclosure domain.

**Pre-admission requirement (normative):** Any TX type whose payload carries
user-identifiable data (sender, receiver, amount, asset type, or
graph-inferrable metadata) MUST declare a receipt encryption scheme in its spec
section BEFORE genesis admission.  Specifically, the TX spec must include:

```yaml
receipt_privacy:
  body_encryption: <scheme name and key derivation>
  disclosure_domain: <genesis-authorised scope>
  plaintext_at_halt: false   # receipt bodies must not auto-decrypt on halt
```

A TX type that omits this declaration and carries user-identifiable payload data
is inadmissible.  No post-genesis governance process may retroactively modify
receipt privacy properties.

**Current state:** No Domain A receipt struct exists yet (`06_receipts.md` is
deferred).  The pre-admission requirement binds the future receipt spec design.

---

## §P8 — OEM/blinding trust boundary

OEM/TEE protection is a side-channel hardening boundary, not a trusted hardware
assumption for privacy.  Cryptographic privacy guarantees hold even on untrusted
hardware because all Domain A operations use deterministic blinding (epoch-bound
PRF masks; no TRNG in Domain A).

**Current Lyapunov factors** (implemented in `crates/consensus/src/lyapunov.rs`):
`divergence_D`, `conflict_C`, `slash_accum_Σ`.  These cover consensus
correctness and validator accountability.

**`blinding_health` Lyapunov factor: NOT YET IMPLEMENTED.**  This is a DEFERRED
Domain B spec item.  When the Domain B blinding spec is written, it must:

1. Define a `blinding_health` metric and its valid range.
2. Specify the weight and update rule.
3. Add it to the Lyapunov evaluation path in both the spec and `lyapunov.rs`.

Until then, §P8 states the normative target: hardware attestation failure or
detected blinding compromise MUST trigger an absorbing halt, not degradation to
a leaky state.  The mechanism for detection is deferred; the halt requirement is
not.

---

## §P9 — Offline clone privacy

The clone protocol (QR/NFC/BLE, `max_offline_epochs = 12`) allows state
transitions to accumulate privately before public network commitment.  The
admission invariant requires that offline state deltas satisfy all transition
axioms; the individual transaction history inside the delta is not required to be
disclosed publicly.

Hop depth is not restricted at the protocol layer.  Enforcement of hop-depth
limits, if any, is an OEM/deployment-specific Domain B concern.

---

## §P10 — Theorem targets

The following theorems are normative targets.  Coq proof skeletons are deferred
to the Domain B spec revision.

```
TH-P1 (Public graph non-observability):
  For any two admissible transaction sequences T_a, T_b yielding identical
  epoch count and halt status, PublicTranscript(T_a) and PublicTranscript(T_b)
  are computationally indistinguishable under CPA, differing only in
  state_root and receipt_root commitments. No semantic transaction graph,
  adjacency structure, or economic metadata is externally observable.

TH-P2 (Receipt non-disclosure):
  receipt_root commits to blinded execution traces via the hash cascade.
  Without a genesis-authorised disclosure capability, no party can extract
  sender, receiver, amount, action type, or graph adjacency from receipt
  data or membership proofs.
```

**TH-P1 dependency chain:**

- Domain B blinding implementation (deterministic PRF masks; no TRNG in Domain A)
- Cascade avalanche property: `proofs/privacy/cascade_avalanche_property.v` (deferred)
- ORAM/dummy access pattern correctness:
  `proofs/privacy/oblivious_access_non_interference.v` (deferred)

**TH-P2 dependency chain:**

- Receipt encryption scheme (`06_receipts.md`, deferred)
- Disclosure key management spec (deferred)
- ZK membership proof soundness:
  `proofs/privacy/receipt_proof_soundness.v` (deferred)

The proof file paths above are reserved names.  A genesis proposal that lacks
TH-P1 and TH-P2 evidence is inadmissible.
