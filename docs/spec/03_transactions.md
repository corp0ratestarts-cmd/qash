# QASH Transaction Semantics
## `docs/spec/03_transactions.md` — Protocol Version 1.0

> **Authority notice:** The QASH v1.0 PDF in `spec/pdf/QASH_Spec_v1.0.pdf`
> is the normative source of truth once checked in. This file is a pre-existing
> engineering specification and must be treated as derived/non-normative unless
> a traceability row, erratum, or ADR explicitly elevates a requirement.
> See `docs/traceability.md`.


> **Status:** Derived engineering specification. It is constrained by the
> normative PDF, accepted errata, and accepted ADRs.
> Transaction rules become genesis-binding only after traceability review.

---

## Purpose and Position in the Spec Stack

```
00_execution_model.md   — computational substrate law
01_consensus.md         — state space, encoding, stability functions
02_transition_axioms.md — admissible transformation law (axioms A0-A11)
03_transactions.md      ← you are here: concrete transaction instantiations
```

This document defines concrete transaction types as instantiations of the
abstract transition algebra defined in `02_transition_axioms.md`.

A transaction type exists in this protocol if and only if:
1. Its admissibility predicate satisfies §A0–A7 of `02_transition_axioms.md`
2. It declares one of Form A, B, or C from §A8 and a proof obligation exists
3. It is listed in Appendix A (Transaction Type Registry) at genesis

The registry is **closed** at genesis. New transaction types require a new network.

---

## Notation (aligned with `02_transition_axioms.md`)

| Symbol | Meaning |
|--------|---------|
| `τ` | A single canonical transaction (instance) |
| `TX-k` | Transaction type k (class) |
| `Encode(τ)` | Canonical encoding of τ (§1 below) |
| `TxID(τ)` | `H_domain(TX_ID, Encode(τ))` — stable replay identity |
| `sort_key(τ, S_t)` | Canonical ordering key for τ at state `S_t` (§4) |
| `𝒜_τ(S_t, τ)` | Per-type admissibility predicate (§2) |
| `𝒯_τ(S_t, τ)` | Per-type transition function (§3) |
| `touch(τ)` | Set of state fields τ may modify (§3) |
| `ε_τ` | Perturbation budget declared by τ (§A8 of 02) |
| `σ_τ` | Slash increment bound declared by τ (§6) |
| `prior_root(t)` | Defined in `01_consensus.md §2` |
| `Encode_for_commitment` | Defined in `01_consensus.md §2` |
| `H_domain(tag, ...)` | Domain-separated hash (`00_execution_model.md §E4`) |

All arithmetic obeys `00_execution_model.md §E1`.
All encoding obeys `01_consensus.md §2`.

---

## §0 — Transaction Model

### Transaction universe

The set of admissible transaction types is finite and frozen at genesis:

```
𝕋 = TX-0 ∪ TX-1 ∪ ...   (closed set; recorded in Appendix A)
```

Each `TX-k` is a **transition algebra**, not an application protocol.
It defines:
- A canonical payload schema (`§1`)
- An admissibility predicate `𝒜_τ` (`§2`)
- A pure transition function `𝒯_τ` (`§3`)
- Declared bounds `ε_τ` and `σ_τ`
- A §A8 form (A, B, or C) with a proof obligation file

### Ontology invariant

No transaction may introduce new encoding rules, new hash domain tags, or
new arithmetic semantics. All transactions inherit:

- Canonical encoding from `01_consensus.md §2`
- Fixed-point arithmetic from `00_execution_model.md §E1`
- Commitment semantics from `01_consensus.md §2`
- Halt semantics from `00_execution_model.md §E5`
- Domain separation from `00_execution_model.md §E0`

### Transaction lifecycle

```
1. Decoding:     bytes → Tx envelope or DecodeResult::Invalid(reason)
2. Admission:    τ ∈ I_t iff 𝒜_τ(S_t, τ) = true
3. Ordering:     I_t.transactions sorted by sort_key(τ, S_t) ascending
4. Application:  S' = 𝒯_τ(S_t, τ); state mutation atomic
5. Replay:       guaranteed by nonce advancement (per author validator)
```

---

## §1 — Canonical Transaction Encoding

### Envelope structure (frozen)

```
Tx = (
  version:        u16,          // must equal genesis tx_version = 1
  tx_type:        u16,          // ∈ frozen registry (Appendix A)
  nonce:          u64,          // per-author monotone counter
  author_id:      [u8; 48],     // stable consensus identity (V_i.id)
  payload_len:    u32,          // ≤ max_tx_payload_bytes (genesis)
  payload:        [u8; payload_len],   // type-specific, defined per TX-k
  signature:      Dilithium5Signature, // 2420 bytes, canonical encoding
)
```

### Encoding rules

- All integers: little-endian, explicit width
- No padding, no alignment gaps
- Only `payload` is variable-length; everything else is fixed-width
- `payload` is opaque to the envelope; semantics defined per `tx_type`
- `signature` covers: `Encode(envelope_without_signature)`
- Total envelope size = `89 + payload_len + 2420` = `2509 + payload_len` bytes

### Envelope wire format

```
Encode(τ):
  encode_u16(τ.version)         →   2 bytes
  encode_u16(τ.tx_type)         →   2 bytes
  encode_u64(τ.nonce)           →   8 bytes
  τ.author_id                   →  48 bytes verbatim
  encode_u32(τ.payload_len)     →   4 bytes
  τ.payload                     →  payload_len bytes verbatim
  τ.signature                   →  2420 bytes (Dilithium5 canonical)
                                 = 2484 + payload_len bytes total
```

Note: this is the **wire encoding for transactions**, not for state.
The state encoding is defined in `01_consensus.md §2`. Both are canonical
within their respective domains and never mixed.

### TxID computation

```
TxID(τ) = H_domain(TX_ID = 0x00000010, Encode(τ))
```

`TxID(τ)` is the stable replay identity of τ. It is:

- Used for deduplication in `I_t` (§7)
- Used as secondary sort key in `sort_key(τ, S_t)` (§4)
- Independent of the author's nonce (different nonces → different `TxID`s
  because nonce is in the envelope)

### Decode rejection rules

A byte sequence fails to decode (returns `DecodeResult::Invalid(reason)`)
if any of the following holds. State is unchanged on decode failure.

```
0x10  malformed envelope (wrong total length given payload_len)
0x11  version ≠ 1 (genesis tx_version)
0x12  tx_type ∉ Appendix A registry
0x13  payload_len > max_tx_payload_bytes (genesis constant)
0x14  signature verification fails under author_id's public key
0x15  type-specific decode failure (defined per TX-k below)
```

Rejection reason codes are part of the canonical state. All authorized
platforms must produce identical reason codes for identical inputs
(see `00_execution_model.md §E0` oracle contract for signature verification).

---

## §2 — Transaction Admissibility

### General admissibility predicate

For any well-decoded transaction τ at state `S_t`:

```
𝒜_τ(S_t, τ) :=
  τ is well-decoded                                 (envelope valid per §1)
  ∧ τ.version = 1
  ∧ τ.tx_type ∈ frozen_registry                     (Appendix A)
  ∧ author(τ) ∈ active_validators(S_t)
  ∧ τ.nonce = expected_nonce(S_t, τ.author_id)      (replay protection)
  ∧ payload_admissible(τ.tx_type, S_t, τ.payload)   (per-type, defined below)
  ∧ ε_τ ≤ remaining_epoch_budget(S_t)               (§A8 budget; §5 below)
```

### Admissibility class

If `𝒜_τ(S_t, τ) = false`, the transaction returns
`AdmissibilityResult::Reject(reason)` with state unchanged and the author's
nonce **not advanced**. This is distinct from `⊥` (absorbing halt).

Reason code ranges (canonical, cross-platform identical):

```
0x20 – 0x2F  envelope-level admissibility failures (version, type, signer)
0x30 – 0x3F  state-level admissibility failures (nonce, budget, active set)
0x40 – 0x4F  per-type payload admissibility failures (defined per TX-k)
```

### Failure ontology recap

Three distinct rejection layers, each cross-platform deterministic:

| Layer | Result | Trigger | State effect | Nonce |
|-------|--------|---------|--------------|-------|
| Decode | `DecodeResult::Invalid(0x1X)` | Malformed bytes | unchanged | not advanced |
| Admissibility | `AdmissibilityResult::Reject(0x2X/0x3X/0x4X)` | Valid bytes, invalid transition | unchanged | not advanced |
| Invariant | `⊥` (absorbing halt) | Checked-arithmetic overflow, touch-set violation, halt condition | halted | meaningless after halt |

### Nonce semantics

```
expected_nonce(S_t, author_id) :=
  let V = validator_by_id(S_t, author_id) in V.nonce

apply succeeds → V.nonce := V.nonce + 1   (author's nonce only)
apply rejects  → V.nonce unchanged
```

Nonce advancement binds to the **author** (signer), not any target validator
a transaction may reference in its payload. This is the replay protection
contract — see `01_consensus.md §1` for the validator nonce field.

---

## §3 — Transaction Application Semantics

### Transition function contract

For each `TX-k`, the type-specific transition function `𝒯_τ` must satisfy:

```
𝒯_τ : (S_t, τ) → S_{t+1} | ⊥

Required properties (proof obligations, even if trivial):
  1. Purity:        no side effects outside the returned state
  2. Totality:      if 𝒜_τ(S_t, τ) = true, then 𝒯_τ returns S_{t+1} ≠ ⊥
  3. Confinement:   only fields in touch(τ) are modified
  4. Determinism:   identical (S_t, τ) → identical S_{t+1} on all Ξ
  5. Arithmetic:    all ops obey 00_execution_model.md §E1; overflow → ⊥
```

### Touch-set declaration

Each `TX-k` MUST statically declare its `touch(τ)`: the exact set of state
fields it may modify. Any write outside `touch(τ)` is an invariant violation
and triggers `⊥` (absorbing halt), regardless of admissibility.

Fields **permanently immutable** by any transaction (`01_consensus.md §1`):

```
epoch          (advanced by epoch transition, not by τ)
state_root     (computed last in transition step 9)
halt_reason    (may be set to non-zero, never cleared — §A6 of 02)
validator[i].id (stable consensus identity)
```

### Effect declarations

Each `TX-k` must declare its effect on convergence/safety components:

```
ΔD_i,t(τ) : [0, p]      // change to validator i's divergence (𝔽_p units)
ΔC_i,t(τ) : [0, p]      // change to validator i's conflict (𝔽_p units)
ΔΣ_i,t(τ) : [0, INT_MAX] // change to validator i's slash accumulator
Δentropy(τ) : bool      // does τ advance entropy_seed?
Δledger(τ)  : bool      // does τ modify ledger_root?
```

These declarations are used to derive `ε_τ` (§5) and verify §A8 conformance.

---

## §4 — Deterministic Ordering

### Sort key definition

For any transaction τ admitted at state `S_t`:

```
sort_key(τ, S_t) := (
  primary:   H_domain(ENTROPY_ADVANCE = 0x00000002,
                       S_t.entropy_seed ∥ Encode(τ)),
  secondary: TxID(τ)
)
```

### Ordering relation

```
τ₁ < τ₂  iff  sort_key(τ₁, S_t) <_lex sort_key(τ₂, S_t)
```

where `<_lex` is lexicographic ordering on `(bytes32, bytes32)` (the byte
sequences compared LSB-first as unsigned).

### Uniqueness guarantee

Because `TxID(τ) = H_domain(TX_ID, Encode(τ))` and `Encode` is injective
(per TH-1 from `proofs/contractivity/encode_injectivity.v`), no two distinct
transactions can share both primary and secondary sort keys. The total
ordering is unambiguous.

### Admission ordering invariant

```
I_t.transactions MUST be sorted by < before admission into Domain A.
```

An `I_t` whose transactions are not in canonical order is **inadmissible**
and triggers absorbing halt (`⊥`). This is non-negotiable: differently-ordered
but semantically equivalent batches must not produce divergent state.

This is the operational realization of the canonical ordering axiom in
`02_transition_axioms.md §A10`.

---

## §5 — Stability Budget System (§A8 Interface)

### Perturbation budget declaration

Each transaction type `TX-k` declares a worst-case Lyapunov excursion:

```
ε_τ ∈ 𝔽_p   // static; declared at genesis per tx_type; immutable
```

This is a **static upper bound**, not a runtime computation. It must satisfy:

```
∀ admissible (S_t, τ):
  |V_convergence(𝒯_τ(S_t, τ)) − V_convergence(S_t)| ≤ ε_τ
```

This is the §A8 Form B obligation. Form A is the special case `ε_τ = 0`.
Form C is `ε_τ = 0` plus a separate Φ_safety obligation.

### Epoch budget constraint

For any admissible input set `I_t`:

```
Σ_{τ ∈ I_t.transactions} ε_τ ≤ ε_honest
```

where `ε_honest = 2000` is the convergence proof target from
`01_consensus.md §5`.

This matches the two-threshold model defined in `02_transition_axioms.md §A8`:
`ε_honest = 2_000` is the proof target; `ε_halt = 20_000` is the halt trigger
with a 10× safety margin. TX-k budgets must sum to ≤ ε_honest per epoch.

### Budget accounting function

```
remaining_epoch_budget(S_t) := ε_honest − Σ_{τ already applied this epoch} ε_τ
```

Used in `𝒜_τ` (§2) to reject any transaction whose `ε_τ` would exceed the
remaining budget. Such rejection returns `AdmissibilityResult::Reject(0x32)`
("epoch ε-budget exceeded"); state and nonce unchanged.

### Proof obligation

For each `TX-k`, a proof file at
`proofs/contractivity/tx_perturbation_k.v` must discharge:

```
Theorem TXk_perturbation_bound :
  ∀ S_t τ, 𝒜_τ S_t τ →
    |V_convergence (𝒯_τ S_t τ) − V_convergence S_t| ≤ ε_τ.
```

Until discharged, `ε_τ` values are marked `STATUS: PLACEHOLDER` in
Appendix A. Genesis lock requires all admitted TX-k to have proved bounds.

---

## §6 — Slash Semantics

### Slash increment bound

Each `TX-k` that produces slash increments declares:

```
σ_τ ∈ [0, INT_MAX]   // declared at genesis per tx_type; immutable
```

Must satisfy:

```
∀ admissible (S_t, τ), ∀ affected validator i:
  Σ_i,t(𝒯_τ(S_t, τ)) − Σ_i,t(S_t) ≤ σ_τ
```

This composes with `Σ_i` saturation at `INT_MAX` defined in
`01_consensus.md §4b`. The pre-checked cap rule there ensures intermediate
i128 arithmetic does not overflow.

### Transactions with σ_τ = 0

A transaction that does not produce slash evidence declares `σ_τ = 0` and
need not include slash-related fields in its payload. TX-0 is such a
transaction.

### Slash-producing transactions

The first slash-producing transaction (`TX-2`) is NOT defined in this
revision. Its design will exercise:

- Evidence encoding (canonical, replay-safe)
- Adjudication function (pure, deterministic)
- Deduplication (same evidence → no double-counting)
- Idempotent evidence handling

`TX-2` will be added only after `TX-0` and `TX-1` are stable and CI-verified.

---

## §7 — Replay and Idempotence

### Idempotence law

For any admissible transaction τ:

```
Let S' = 𝒯_τ(S_t, τ)  (first application succeeds).
Then 𝒯_τ(S', τ) = AdmissibilityResult::Reject(0x31)  (nonce mismatch).
```

This holds because applying τ advances the author's nonce, so τ's `nonce`
field no longer matches `expected_nonce(S', τ.author_id)`. The second
application fails admissibility before reaching `𝒯_τ`.

### Replay safety

Combined with TH-7 (replay invariance), the idempotence law implies:

```
Replay(G, T) on any authorized Ξ produces identical state, AND
no transaction is applied more than once.
```

This is replay safety, not just replay determinism.

### Deduplication in input sets

If `I_t.transactions` contains two distinct entries τ₁ and τ₂ with
`TxID(τ₁) = TxID(τ₂)`, the input set is malformed and triggers `⊥`.

Note: `TxID` includes the nonce, so the same transaction submitted with
different nonces produces different `TxID`s. Genuine duplicates can only
arise from byte-identical entries.

---

## §TX-0 — No-Op Transaction

### Purpose

`TX-0` is the minimal admissible transaction. It produces no state change
other than advancing the author's nonce. It serves three roles:

1. **Heartbeat / liveness**: authors can prove activity without state mutation
2. **Test vector baseline**: encoding/ordering/admissibility round-trips
3. **Convergence proof base case**: the trivial §A8 Form A proof obligation

It is not optional. Every QASH network includes TX-0 as the empty case
of the transaction algebra.

### Payload schema

```
TX-0 payload = ∅   (payload_len = 0)
```

The envelope still includes the nonce and author_id; only `payload` is empty.

Total encoded TX-0 size: `2484 + 0 = 2484 bytes`.

### Type-specific decode failures

```
0x15  TX-0 payload_len ≠ 0  (TX-0 must have empty payload)
```

### Admissibility predicate

```
payload_admissible(0, S_t, payload) :=
  payload = []   (empty payload bytes)
```

There are no further per-type constraints. TX-0 is admissible whenever:

- The envelope decodes (§1)
- Author is active in `S_t`
- `τ.nonce = expected_nonce(S_t, τ.author_id)`
- Remaining epoch budget ≥ 0 (trivially satisfied since `ε_τ = 0`)

### Transition function

```
𝒯_TX0(S_t, τ):
  // Precondition: 𝒜_τ(S_t, τ) = true
  
  let author_idx = index_of_validator(S_t, τ.author_id)   // O(1) via fixed array
  
  S_{t+1} = S_t with:
    validators[author_idx].nonce ← validators[author_idx].nonce + 1
    // No other field changes.
  
  return S_{t+1}
```

### Touch-set

```
touch(TX-0) = { validators[author_idx].nonce }
```

The author's nonce is the only field modified. All other validator fields,
the ledger root, state root, entropy seed, halt flag, epoch counter, and
Lyapunov window remain bitwise identical between `S_t` and `S_{t+1}`.

### Effect declarations

```
ΔD_*,t(τ)   = 0   (no validator's divergence changes)
ΔC_*,t(τ)   = 0   (no validator's conflict changes)
ΔΣ_*,t(τ)   = 0   (no slash increments produced)
Δentropy(τ) = false
Δledger(τ)  = false
```

### Declared bounds

```
ε_τ = 0   (TX-0 makes no convergence perturbation)
σ_τ = 0   (TX-0 produces no slash evidence)
```

### §A8 form: Form A (Non-increase)

TX-0 satisfies §A8 Form A with the strongest possible statement:

```
∀ admissible S_t: V_convergence(𝒯_TX0(S_t, τ)) = V_convergence(S_t).
```

This is equality, not merely non-increase. It follows trivially because:

- `V_convergence` is defined over validator divergence and conflict metrics
- TX-0 modifies neither divergence nor conflict (only nonce)
- Therefore `V_convergence` is unchanged

### Proof obligation

```
File:    proofs/contractivity/tx_perturbation_0.v
Theorem: TX0_perturbation_bound

Statement (active Coq model):
  ∀ validator nonce_next window_min,
    δ_window(𝒯_TX0(validator, nonce_next), window_min)
      ≤ δ_window(validator, window_min).

Proof sketch:
  By touch-set confinement, only validators[author_idx].nonce changes.
  V_convergence is defined as Σ_i (α·D_i + β·C_i); it does not reference nonce.
  Therefore V_convergence is invariant under TX-0 application.

Status: FORMAL — proofs/contractivity/tx_perturbation_0.v; zero Admitted
```

### Idempotence

Per §7, applying the same TX-0 twice rejects on the second attempt:

```
Apply TX-0 with nonce = n → author.nonce becomes n+1
Apply same TX-0 (nonce = n) again → AdmissibilityResult::Reject(0x31)
                                     (expected_nonce is now n+1)
```

### Rejection taxonomy specific to TX-0

| Layer | Reason | Trigger |
|-------|--------|---------|
| Decode | 0x10 | malformed envelope |
| Decode | 0x14 | signature verification failed |
| Decode | 0x15 | payload_len ≠ 0 |
| Admissibility | 0x30 | author not in active set |
| Admissibility | 0x31 | nonce mismatch |
| Admissibility | 0x32 | epoch ε-budget exceeded (impossible for TX-0 since ε_τ = 0) |

### Test vectors

```yaml
tv_tx0_basic:
  description: Valid TX-0 with author at nonce 0; should advance to nonce 1
  S_t:
    validators:
      - id:     "0xa1b2...[48 bytes]"   # author
        nonce:  0
        active: true
        score:  100
        divergence: 0
        conflict:   0
        slash_acc:  0
  τ:
    version:    1
    tx_type:    0
    nonce:      0
    author_id:  "0xa1b2...[48 bytes]"
    payload_len: 0
    payload:    []
    signature:  "0x...[2420 bytes]"   # valid Dilithium5 over envelope
  expected:
    result: success
    S_prime.validators[0].nonce: 1
    all_other_fields_unchanged: true
    epsilon_consumed: 0

tv_tx0_replay:
  description: Re-applying same TX-0 fails admissibility (nonce advanced)
  S_t: <state after tv_tx0_basic above>
  τ:   <same as tv_tx0_basic.τ>
  expected:
    result: AdmissibilityResult::Reject(0x31)
    state_unchanged: true

tv_tx0_nonempty_payload:
  description: TX-0 with payload_len > 0 fails decode
  τ:
    version:    1
    tx_type:    0
    nonce:      0
    author_id:  "0xa1b2...[48 bytes]"
    payload_len: 1
    payload:    [0x00]
    signature:  "0x...[2420 bytes]"
  expected:
    result: DecodeResult::Invalid(0x15)
    state_unchanged: true
    nonce_not_advanced: true

tv_tx0_inactive_author:
  description: TX-0 from inactive validator fails admissibility
  S_t:
    validators:
      - id:     "0xa1b2...[48 bytes]"
        nonce:  0
        active: false   # inactive
  τ: <valid TX-0 envelope from this author>
  expected:
    result: AdmissibilityResult::Reject(0x30)
    state_unchanged: true
```

---

## §TX-1 — Validator-Metric Update

*Specification deferred. TX-1 design captured in repository review notes;
not yet ratified into the protocol. TX-1 will be added in a separate spec
revision after TX-0 is CI-validated.*

---

## Appendix A — Transaction Type Registry

This registry is **frozen at genesis**. Adding or removing types requires
a new network.

| tx_type | Name | §A8 form | ε_τ | σ_τ | touch fields | Proof status |
|---------|------|----------|-----|-----|--------------|--------------|
| 0 | TX-0 No-Op | A | 0 | 0 | `{validators[author].nonce}` | FORMAL |

Future revisions will extend this table as transaction types are ratified.

---

## Appendix B — Cross-Document Dependency

| Reference | Document | Section |
|-----------|----------|---------|
| Canonical encoding | `01_consensus.md` | §2 |
| State space | `01_consensus.md` | §1 |
| Stability functions | `01_consensus.md` | §4a, §4b |
| Stability criterion | `01_consensus.md` | §5 |
| Arithmetic model | `00_execution_model.md` | §E1 |
| Domain partition | `00_execution_model.md` | §E0 |
| Oracle contract (signatures) | `00_execution_model.md` | §E0 |
| Hash domain tags | `00_execution_model.md` | §E4 |
| Halt semantics | `00_execution_model.md` | §E5 |
| Transition axioms | `02_transition_axioms.md` | §A0–A11 |
| §A8 forms | `02_transition_axioms.md` | §A8 |
| Canonical ordering | `02_transition_axioms.md` | §A10 |

---

*End of `docs/spec/03_transactions.md`*
*SHA3-256 of this document is recorded in `GENESIS_CONSTANTS.toml` at genesis lock.*
