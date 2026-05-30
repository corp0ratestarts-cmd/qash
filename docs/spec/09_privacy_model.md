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

## §P0a — What QASH does NOT protect against (normative)

| Threat | Why out of scope | Mitigation (user responsibility) |
|--------|-----------------|----------------------------------|
| Endpoint compromise | Cryptographic privacy holds only if keys are not extracted. TEE/OEM halts before responding, but hardware extraction is outside protocol scope. | Hardware-backed keys; attestation monitoring. |
| Physical coercion | Outside cryptographic scope. | User responsibility. |
| Exchange/egress correlation | Economic layer, not consensus. When validators exchange external-network assets, that layer may expose graph edges. | Out-of-band disclosure controls. |
| Global passive adversary (full network view) | QASH's privacy is architectural (no graph broadcast), not transport-layer anonymity. A GPA watching all Domain B channels may correlate sync timing. | Optional Tor routing for clone channels (Domain B feature gate). |
| Receipt body without disclosure key | By design: requires a genesis-authorized disclosure key. | This is the privacy guarantee, not a limitation. |

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
  efb_root_t,
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

`efb_root_t` is a public commitment over already-public shard state roots,
receipt roots, the previous EFB root, shard count, and optional transparent
proof batch root. It is permitted in `PublicTranscript` because it does not
publish raw receipt leaves, transaction envelopes, admission transport metadata,
or edge topology.

### §P3a — PublicTranscript change-control process (normative)

`PublicTranscript` is the sole authorised pathway for Class I–visible data.
Any code path that emits protocol state to a public-observable channel MUST
go through `qash_consensus::public::PublicTranscript`.  Raw `EpochState`
MUST NOT be serialised and transmitted directly.

**Gating rule:** Any PR that adds a new field to `PublicTranscript` or widens
the public surface in any other way MUST satisfy all of the following before
merge:

1. **Add the field** to `crates/consensus/src/public.rs:PublicTranscript`.
2. **Add a COVERAGE.md row** explaining the privacy implication: what the field
   reveals, which observer class gains access, and why the privacy cost is
   acceptable.
3. **Receive explicit sign-off** in the PR from a designated privacy reviewer
   (tracked in `CODEOWNERS` as the `@privacy-reviewers` team).
4. **Update §P4** in this document to reflect the widened class boundary, if
   applicable.

A PR that widens the public surface without these steps MUST be blocked at
review.  This requirement is not waivable post-genesis.

**Domain B emission gate:** The only function in Domain B permitted to write
to a public channel is:

```rust
// crates/pal/src/net.rs (normative target, implementation pending 4-B)
pub fn publish_transcript_entry(
    transport: &impl NetTransport,
    entry: &PublicTranscript,
) -> Result<(), NetError> {
    let bytes = entry.encode_canonical();
    transport.broadcast(&bytes)
}
```

This function is intentionally NOT generic over `T`.  If you need to publish
raw `EpochState`, that is a privacy violation.

---

## §P4 — Observer classes

### §P4a — Normative class taxonomy (Phase 4-A)

QASH defines four normative observer classes.  Each class has an exhaustive list
of visible artefacts and a hard boundary on what it MUST NOT see.  Any protocol
change that widens a class boundary (makes previously invisible data visible) is
a privacy regression and requires the sign-off process described in §P3a.

**Class I — Public observer (unauthenticated internet participant)**

> Can see: `(epoch, state_root, receipt_root, efb_root, halt_flag)`.  
> MUST NOT see: validator identities or counts, transaction amounts, sender/receiver,
> envelope payloads, graph topology, receipt leaf values.

This is the minimal public surface.  `PublicTranscript` in
`crates/consensus/src/public.rs` encodes this as a compile-time type boundary.
Any value not present in `PublicTranscript` is Class I–invisible by construction.

**Class II — Authorized validator (Domain A + B, TEE/OEM-protected)**

> Can see: own validator slot assignment, aggregated divergence metrics (Lyapunov
> `W_D`, `W_C`, `W_Σ`) for the active epoch, blinded opcodes within own shard.  
> MUST NOT see: other validators' private signing keys, envelope plaintext of
> other validators' transactions, exact divergence decomposition of other
> validators' contributions.

Validators are intentionally linkable across epochs — they are accountable
consensus participants.  This is architecturally distinct from user privacy.

**Class III — Receipt holder (scoped Domain B disclosure)**

> Can see: own receipt contents when holding the corresponding epoch viewing key
> (derived from `epoch_seed` via the key-derivation function specified in §P7).  
> MUST NOT see: other participants' receipts, graph adjacency between receipts,
> economic metadata of other participants, or receipt contents from epochs after
> key rotation (forward secrecy).

Receipt decryption requires both the receipt ciphertext and the epoch-scoped
viewing key.  The viewing key is destroyed after `max_offline_epochs` (12,
from `GENESIS_CONSTANTS.toml`) as a forward-secrecy guarantee.

**Class IV — Regulatory authority (genesis-authorised disclosure domain)**

> Can see: specific receipts for which a genesis-authorised disclosure key has
> been scoped and a lawful-basis disclosure request has been satisfied.  
> MUST NOT see: receipts outside the disclosure scope, receipt contents from
> epochs prior to the disclosure key's activation epoch (disclosure is not
> retroactive), or graph topology beyond the explicitly disclosed receipts.

Class IV access requires:
1. A genesis-authorised disclosure key (not derivable post-genesis).
2. A valid lawful-basis disclosure request (GDPR Art. 6/9, national equivalent).
3. Epoch-scoped decryption — the disclosure key is valid only for a declared
   epoch range.  Past-epoch decryption is cryptographically impossible after key
   rotation (`epoch_seed` destruction).

This is the forward-secrecy property for Class IV: a disclosure key authorised
at epoch T cannot decrypt receipts from epoch T−k, even with full regulatory
cooperation.

### §P4b — Observer class summary table

| Class | Name | Domain Access | Can See | MUST NOT See |
|-------|------|---------------|---------|--------------|
| I | Public observer | Domain A only | `state_root`, `receipt_root`, `efb_root`, `epoch`, `halt_flag` | Validator IDs, TX amounts, sender/receiver, graph edges |
| II | Authorized validator | Domain A + B (TEE/OEM) | Own slot, aggregated divergence, blinded opcodes (own shard) | Other validators' private keys, other TX plaintext |
| III | Receipt holder | Scoped Domain B | Own receipts with epoch viewing key | Other receipts, graph adjacency, past-epoch receipts after key rotation |
| IV | Regulatory authority | Genesis-authorised Domain B | Disclosed receipts (epoch-scoped, lawful basis) | Out-of-scope receipts, pre-activation-epoch receipts, graph topology |
| — | Compromised endpoint | Out of protocol scope | — | TEE/OEM concern; protocol halts before degrading |

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
