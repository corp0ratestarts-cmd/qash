# QASH Design Decisions

> **Status:** Architectural decisions record. Each entry captures a decision
> point where multiple coherent designs were possible, the choice made, and
> the rationale. These decisions constrain all downstream specs.
>
> Modifying a decision in this document requires re-evaluating every downstream
> spec that depends on it. Decisions are not edited in place — they are
> superseded by a new entry referencing the old one.

---

## Project Identity

QASH is a **deterministic replicated transition calculus** designed to be
the digital equivalent of physical cash, with properties that exceed cash:

| Property | Physical cash | QASH |
|----------|---------------|------|
| Offline operability | yes | yes (clone protocol, DD-2) |
| Jurisdictional neutrality | yes | yes (multi-hash cascade, DD-3) |
| No governance | yes | yes (one-shot design, DD-2) |
| Survives material/technology breaks | yes (paper, metal, plastic) | yes (PQC agility, DD-2; hash cascade, DD-3) |
| Cryptographic verifiability | no | yes |
| Deterministic replay across platforms | no | yes (tiered execution, DD-4) |
| Auditability without trust | partial (serial numbers) | yes (receipts, DD-5) |

This framing resolves apparent tensions in the protocol: every design choice
that seemed in conflict with "deterministic replay" or "one-shot design" is
actually a property cash already has, restated for digital form.

---

## DD-1 — Project Identity is "Transition Calculus", not Blockchain

**Decision:** QASH is a deterministic replicated transition calculus.
Blockchain deployment topology is one of many possible deployment models.
The protocol's primary invariant is replay determinism, not chain consensus.

**Rationale:** Earlier framings of QASH as "a blockchain" created ongoing
tension with the offline / clone / multi-platform / no-governance properties
of the design. None of those tensions exist when QASH is understood as a
transition calculus that *can be deployed as* a blockchain among other things.

**Implications:**
- "Consensus" is one deployment shape; clone protocol is another
- Replay invariance (TH-7) is the central theorem, not chain head agreement
- The state root chain is a commitment structure, not a "block chain"

**Captured in:** `README.md`, `docs/spec/01_consensus.md` framing

---

## DD-2 — Clone Protocol Operates in a Detached State Domain

**Decision:** The replay invariant `∀ Ξ: Replay_Ξ(G,T) = R_n` applies strictly
to the canonical online ledger. The clone protocol operates in a formally
bounded `DetachedStateDomain`. Reconciliation is a **constrained admission
process, not a merge**.

**Resolution:** When a detached node rejoins:
1. Compute state-diff against the canonical chain tip
2. Verify admission criteria:
   - Hop count ≤ 7
   - Epoch offset ≤ 12
   - Valid cryptographic signatures on all state transitions in the diff
   - No double-spend, invalid opcode, or out-of-bounds state violations
3. If all criteria pass: apply diff deterministically via
   `Reconcile(S_detached, S_canonical) → S_admitted`
4. If any criterion fails: discard the detached state, node resyncs from
   canonical tip

The core replay invariant remains uncompromised: clone state is a derivative
admission candidate, not a parallel truth. The canonical ledger never forks.

**Why these specific limits:**
- **7 hops:** beyond this depth, replay/sybil probability compounds beyond
  the protocol's cryptographic guarantees
- **12 epochs:** beyond this offset, reconciliation cost outweighs the
  utility of reintegrating the detached branch

**Implications:**
- A new spec section will define `DetachedStateDomain` formally
- `Reconcile()` is a pure deterministic function (Domain A)
- Transport mechanisms (QR/NFC/BLE) are Domain B
- Clone state is signed and hop-counted at every transition
- A node holding clone state is in a known protocol mode, not "offline"

**Captured in:** future `docs/spec/05_clone_protocol.md`
**Depends on:** DD-4 (tiered execution — most clone-capable devices are Tier B)

---

## DD-3 — Multi-Hash Cascade: SHA3-256 is Canonical; Others are Audit Commitments

**Decision:** SHA3-256 is the **sole canonical consensus hash**. BLAKE3,
KangarooTwelve, SM3, and Streebog are **parallel audit/jurisdictional
commitments**, not co-consensus hashes.

**Resolution:**

Consensus validity requires only `verify(SHA3-256(state_data))`. The four
secondary hashes are computed in parallel, Merkleized into an
`audit_commitment_root`, and included in the block header.

Secondary hash validation is **profile-enforced**, not consensus-enforced:

| Node profile | SHA3-256 | Secondary hashes |
|--------------|----------|------------------|
| Full consensus | enforce | log mismatches, do not reject |
| Compliance / audit | enforce | enforce required secondary; reject persistent-mismatch peers |
| Light client | verify proof | optional, profile-defined |

**Why this works:**
- Consensus determinism preserved (one hash governs)
- Replay invariance unaffected (TH-1 still proves SHA3-256 injectivity of
  canonical encoding)
- Post-quantum hedging: if SHA3-256 is later compromised, secondary hashes
  provide a verifiable migration anchor for the successor network
- Jurisdictional neutrality: SM3 (China) and Streebog (Russia) allow
  sovereign nodes to verify state under their own primitives
- Audit redundancy: independent verification paths for high-stakes use

**Performance impact:** Secondary hashes are computed in parallel during
block construction. Verification is opt-in per profile. Consensus path
remains single-hash cost.

**Implications:**
- Block header structure expands to include `audit_commitment_root`
- `audit_commitment_root` derivation rule is canonical (deterministic
  Merkleization of the five hashes in a fixed order)
- Profile selection is operator-level, not protocol-level

**Captured in:** future `docs/spec/07_hash_cascade.md`

---

## DD-4 — Tiered Execution: Tier A Native, Tier B via Canonical Reference Interpreter

**Decision:** Two tiers with explicit guarantee contracts.

**Tier A** (formal, native): `x86_64-avx2`, `aarch64-neon`, `riscv64-vector`
- Native VM execution
- Formally verified against TH-7
- Replay invariance proved across this set

**Tier B** (canonical interpreter): everything else in `authorized_platforms`
- Executes Tier A bytecode through a deterministic Canonical Reference
  Interpreter (CRI) specification
- Strict requirements: fixed-point math, no UB, deterministic RNG,
  IEEE 754 strict compliance where floats are involved
- Subject to TH-7b: `∀ input ∈ valid_bytecode: Exec_Native(input) == Exec_CRI(input)`

**Resolution:**

```
TH-7   (Tier A): ∀ Ξ ∈ {x86_64-avx2, aarch64-neon, riscv64-vector}:
                   Replay_Ξ(G,T) = R_n

TH-7b  (Tier B): ∀ bytecode B, ∀ ΞB ∈ Tier B platforms:
                   Exec_CRI(B, ΞB) = Exec_Native(B, ΞA) for any ΞA ∈ Tier A
```

Tier B guarantees **identical final state**, not identical performance or
memory footprint. Tier B nodes that detect state-diff divergence halt and
resync.

**Why tiered, not uniform:**
- Identical native execution across 12+ ISAs is mathematically and
  practically infeasible (varying SIMD widths, calling conventions,
  endianness, floating-point quirks)
- Deployment neutrality is a hard requirement (clone protocol assumes
  phones, embedded devices, sovereign hardware)
- The CRI translation layer is itself proof-bounded — translation is not
  the same as interpretation in the bad sense

**Implications:**
- `GENESIS_CONSTANTS.toml` must explicitly tier each authorized platform
- CRI specification is a normative spec document
- TH-7b is a proof obligation (Coq/Isabelle equivalence proof)
- Runtime attestation distinguishes Tier A from Tier B
- Tier B nodes participate in consensus but with explicit downgrade paths
  on divergence detection

**Captured in:** future `docs/spec/04_tiered_execution.md` (next document)

---

## DD-5 — Receipts are Consensus State, Excluded from state_root for Performance

**Decision:** Receipts are **consensus state**. They are excluded from
`state_root` for performance, but their Merkle root (`receipt_root`) is
included in the block header and validated by all full nodes.

**Resolution:**

Block header contains three roots:

```
state_root              — canonical protocol state (R_t per 01_consensus.md)
receipt_root            — deterministic execution traces, Merkleized
audit_commitment_root   — secondary hash side-commitments (DD-3)
```

- Full nodes validate `receipt_root` against actual execution traces.
  Mismatch = invalid block = validator slash (TX-2 territory when defined)
- Light clients verify receipts via `receipt_root` and Merkle proofs
- Pruning policy is separate from consensus validity: receipts remain
  consensus-bound but archival is operator-policy

**Why not in state_root:**
- State root computation is the hot path (every transition)
- Receipts are append-only execution traces; bloating state_root with
  them would slow every operation, not just receipt-bearing ones
- Receipts are read-heavy, write-once — fits a separate Merkle tree better
- Light clients verifying a single receipt should not need to traverse
  the entire state tree

**Why consensus state, not off-chain:**
- Validators must agree on receipt content (deterministic execution
  traces are part of "what happened")
- Disagreement on receipts is a consensus failure, not an audit anomaly
- Off-chain receipts would create a parallel trust model — incompatible
  with QASH's no-governance, no-out-of-band-coordination ethos

**Implications:**
- Receipt schema is normative (canonical encoding of execution traces)
- Receipt generation is part of the transition function
- Slash semantics expand: incorrect `receipt_root` is a slashable offense
- Light client verification flow is a normative spec

**Captured in:** future `docs/spec/06_receipts.md`

---

## DD-6 — PQC Agility is a Pre-Baked, Deterministic Schedule

**Decision:** `pqc_agility_epoch = 10000` is a **deterministic, pre-baked
transition schedule**, not a governance trigger or adaptive migration.

**Resolution:**

`GENESIS_CONSTANTS.toml` contains an **immutable, ordered PQC fallback chain**:

```toml
[crypto.cascade.rotation_schedule]
epoch_0      = "Dilithium5"           # current primary
epoch_10000  = "ML-DSA-87"            # first rotation
epoch_20000  = "SLH-DSA-SHA3-256"     # second rotation
epoch_30000  = "Falcon-512"           # third rotation
epoch_40000  = "TERMINAL_HALT"        # exhaustion → cryptographic halt
```

At each scheduled epoch, the protocol **automatically activates** the next
algorithm. No voting, no soft forks, no runtime parameter changes.

If the entire pre-baked sequence is cryptographically invalidated before
or during activation, the protocol enters a **cryptographic halt**:
consensus stops, state is preserved read-only, and a new network (with a
new genesis) is required. Safety over liveness.

**Why this preserves "one-shot":**
- All agility parameters are committed at genesis
- No runtime decisions are made
- No governance mechanism activates the schedule — only the epoch counter
- Failure mode is halt, not fork

**Why this preserves "zero governance":**
- The successor algorithm at each epoch is pre-decided at genesis lock
- Operators cannot choose, vote on, or accelerate transitions
- If pre-baked algorithms prove insufficient, the network ends —
  successors are new networks, not upgrades

**Implications:**
- `GENESIS_CONSTANTS.toml` must contain the complete rotation schedule
- The choice of pre-baked successor algorithms is irrevocable
- This is the single most consequential choice at genesis lock — it
  commits the network's cryptographic future for its entire lifetime
- A "good lifetime" might be ~30,000 epochs (~5 years at 500ms epochs)
  if all four algorithms survive that long
- Beyond exhaustion or invalidation, succession (TH-8) takes over:
  the next QASH network inherits the prior state_root as anchor

**Captured in:** future `docs/spec/08_pqc_agility.md`

---

## Decision Status Summary

| ID | Decision | Status | Captured in |
|----|----------|--------|-------------|
| DD-1 | Transition calculus identity | accepted | README.md, 01_consensus.md |
| DD-2 | Detached state domain | accepted | future 05_clone_protocol.md |
| DD-3 | SHA3 canonical + audit cascade | accepted | future 07_hash_cascade.md |
| DD-4 | Tiered execution (A native, B CRI) | accepted | future 04_tiered_execution.md (next) |
| DD-5 | Receipts as consensus state | accepted | future 06_receipts.md |
| DD-6 | Pre-baked PQC rotation schedule | accepted | future 08_pqc_agility.md |

---

## Spec Drafting Order (Authoritative)

Based on dependency analysis:

```
1. 04_tiered_execution.md       ← substrate for everything below
2. 05_clone_protocol.md         ← needs DD-4 (Tier B platforms)
3. 06_receipts.md               ← needs DD-4 (light client capabilities)
4. 07_hash_cascade.md           ← independent of execution tier
5. 08_pqc_agility.md            ← needs hash cascade for migration anchor
```

Each document, when drafted, replaces the corresponding "future ..." reference
in the captured-in column above.

---

*End of `docs/design_decisions.md`*
