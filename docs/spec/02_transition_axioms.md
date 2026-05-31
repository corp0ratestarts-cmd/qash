# QASH Transition Axioms
## `docs/spec/02_transition_axioms.md` — Protocol Version 1.0

> **Authority notice:** The QASH v1.0 PDF in `spec/pdf/QASH_Spec_v1.0.pdf`
> is the normative source of truth once checked in. This file is a pre-existing
> engineering specification and must be treated as derived/non-normative unless
> a traceability row, erratum, or ADR explicitly elevates a requirement.
> See `docs/traceability.md`.


> **Status:** Derived engineering specification. It is constrained by the
> normative PDF, accepted errata, and accepted ADRs.
> `03_transactions.md` is constrained by this document where traceability rows
> or accepted ADRs bind these axioms.

---

## Purpose and Position in the Spec Stack

```
00_execution_model.md   — computational substrate law
01_consensus.md         — state space, encoding, stability functions
02_transition_axioms.md ← you are here: admissible transformation law
03_transactions.md      — concrete transaction types (instantiations of axioms)
```

This document defines what **all possible transitions** are permitted to do,
stated as mathematical axioms, before any concrete operation exists.
`03_transactions.md` then defines specific transaction classes as concrete
instantiations of these axioms — each with a proof obligation showing conformance.

A transition is not admissible because it has been implemented.
A transition is admissible because it satisfies every axiom in this document.

---

## Notation

| Symbol | Meaning |
|--------|---------|
| `τ` | A single admissible transition (atomic state transformation) |
| `τ(S_t)` | State resulting from applying τ to S_t |
| `I_t = {τ₁, ..., τₘ}` | The ordered set of transitions in epoch t |
| `ApplyAll(S_t, I_t)` | Sequential application of all τᵢ in canonical order |
| `S_t ⊢ τ` | τ is applicable to S_t (admissibility relation) |
| `δ_window(S_t)` | Rolling-window excursion of V_convergence at S_t (from §5 of 01_consensus.md) |
| `ε_τ` | Bounded perturbation budget for transition class τ |
| `Φ_safety(S_t)` | Monotone safety accumulator (from §4b of 01_consensus.md) |
| `Encode(S_t)` | Canonical wire encoding (from §2 of 01_consensus.md) |
| `⊥` | Absorbing halt (irreversible) |
| `p` | Fixed-point scale = 1_000_000 |

---

## §A0 — Transition Admissibility

A transition `τ` is **admissible** with respect to state `S_t` iff all of the
following hold. Violation of any single axiom makes `τ` inadmissible; an
inadmissible transition applied to any state must trigger absorbing halt.

```
S_t ⊢ τ  iff:

A1.  τ is deterministic over S_t                          (§A1)
A2.  τ terminates within the epoch execution budget        (§A2)
A3.  τ mutates only explicitly declared state fields       (§A3)
A4.  τ preserves canonical encodability of S_{t+1}        (§A4)
A5.  τ preserves replay invariance across Tier A ISAs     (§A5)
A6.  τ does not clear halt_flag                           (§A6)
A7.  τ does not introduce Domain B artifacts into state   (§A7)
A8.  δ_window effect of τ is bounded (see §A8)            (§A8)
```

The admissibility relation is **closed**: a transition that is not proved
admissible is inadmissible by default. There is no implicit permission.

---

## §A1 — Determinism Axiom

> **∀ τ, S_t: if S_t ⊢ τ then τ(S_t) is uniquely determined.**

A transition may not depend on:

```
FORBIDDEN sources of nondeterminism in τ:
  - Memory layout or allocation address
  - Wall-clock time or hardware counters
  - OS scheduler or thread interleaving
  - Allocator order or GC behavior
  - ISA-specific undefined behavior
  - Nondeterministic iteration order (HashMaps, etc.)
  - Any Domain B value not passed through the canonical input I_t
```

A transition is deterministic iff for all admissible S_t, applying τ to S_t
on any two authorized platforms produces bitwise-identical S_{t+1}.

This axiom is the protocol instantiation of the Domain A definition in
`00_execution_model.md §E0`. It is restated here as a transition-level law.

**Proof obligation:** Every transaction type in `03_transactions.md` must
include a determinism argument showing no forbidden sources are reachable.

---

## §A2 — Termination Axiom

> **∀ τ, S_t: if S_t ⊢ τ then τ(S_t) terminates within the epoch budget.**

Every transition must terminate within the epoch execution budget:

```
max_control_loop_latency_ms = 450ms   (from GENESIS_CONSTANTS.toml)
```

More precisely, every transition must be **statically bounded**: the number
of computation steps is bounded by a function of the genesis constants alone,
independent of runtime state.

```
REQUIRED: static step bound B_τ such that
  steps(τ, S_t) ≤ B_τ(N_max, max_queries_per_epoch)
  for all admissible S_t
```

No unbounded recursion. No loops without a static iteration budget.
Divergence is treated as an overflow condition and triggers absorbing halt.

**Proof obligation:** Every transaction type must supply `B_τ` and a proof
that execution stays within it for all admissible inputs.

---

## §A3 — State Locality Axiom

> **∀ τ, S_t: τ(S_t) differs from S_t only in explicitly declared fields.**

Each transaction type in `03_transactions.md` must declare a **mutation
footprint** — the exact set of state fields it may modify:

```
mutation_footprint(τ) ⊆ {
  validators[i].score,
  validators[i].divergence,
  validators[i].conflict,
  validators[i].slash_acc,
  validators[i].active,
  ledger_root,
  entropy_seed   (only via the canonical advance rule in §3 of 01_consensus.md)
}
```

**Permanently immutable fields** — no transition may ever modify:

```
IMMUTABLE under any τ:
  epoch           (advanced only by the epoch transition rule, not by τ)
  state_root      (computed by the transition function, not by τ directly)
  halt_flag       (may be set to true, never cleared — see §A6)
  validator[i].id (identity is fixed at genesis for each validator slot)
```

**Proof obligation:** Every transaction type must declare its footprint.
Any write outside the declared footprint makes τ inadmissible.

---

## §A4 — Encoding Preservation Axiom

> **∀ τ, S_t: if S_t ⊢ τ then Encode(τ(S_t)) is well-defined and canonical.**

This axiom follows from TH-1 (encoding injectivity) but is stated explicitly
as a transition-level constraint: a transition that would produce a state
`S_{t+1}` for which `Encode(S_{t+1})` is undefined or non-canonical is
inadmissible.

Concretely, every field of `τ(S_t)` must satisfy the well-formedness
conditions of `StateWF` (defined in `proofs/contractivity/encode_injectivity.v`):

```
StateWF(τ(S_t)) must hold for all admissible (τ, S_t)
```

If any field after application violates its type bound (e.g. a score field
exceeds `INT_MAX`), the transition triggers absorbing halt before committing.

**Proof obligation:** Every transaction type must include a `StateWF`
preservation lemma.

---

## §A5 — Replay Preservation Axiom

> **∀ τ, S_t, ISA₁, ISA₂ ∈ Tier A: τ(S_t) on ISA₁ = τ(S_t) on ISA₂.**

This is the transition-level instantiation of TH-7 (replay invariance).
Every transition must be replay-invariant across the Tier A ISA set
`{x86_64-avx2, aarch64-neon, riscv64-vector}`.

This axiom is operationally enforced by:
- The `platform-determinism.yml` CI workflow
- The test vector suite (`tests/vectors/README.md` and `tests/vectors/vectors.v1.json`)
- The cross-ISA replay check on every PR

A transition that passes single-ISA tests but fails cross-ISA replay is
inadmissible. CI is the enforcement mechanism; the axiom is the specification.

---

## §A6 — Halt Monotonicity Axiom

> **∀ τ, S_t: halt_flag(τ(S_t)) ≥ halt_flag(S_t).**

In boolean terms: `halt_flag` may be set to `true` but never cleared.

```
if halt_flag(S_t) = false:  τ(S_t).halt_flag ∈ {false, true}
if halt_flag(S_t) = true:   τ(S_t) = ⊥   (absorbing halt, no transition)
```

This axiom formalizes TH-6 at the transition level. A halted state is a
terminal state. No transaction class may include a `clear_halt` operation.
No governance mechanism can override this.

**Proof obligation:** Trivially discharged for any τ that does not touch
`halt_flag`. For τ that may set `halt_flag = true`, no additional obligation
— only that it does not set `halt_flag = false`.

---

## §A7 — Domain Separation Axiom

> **∀ τ: no Domain B artifact influences τ(S_t).**

The precise statement from `00_execution_model.md §E0` applied to transitions:

```
FORBIDDEN in any τ:
  - Wall-clock time as input
  - OS entropy as input
  - Network state as input
  - Hardware attestation results as state-mutating inputs
  - Any value whose provenance is Domain B
```

All inputs to τ must arrive via the canonical input set `I_t`, which is
itself admissibility-checked before `ApplyAll` is called. The domain
boundary is enforced structurally: `I_t` contains only canonically signed,
deterministically ordered transactions. Any out-of-band input is a
boundary violation.

---

## §A8 — δ_window Compatibility Axiom

> **This is the most critical axiom. It is the constitutional law
> governing what transitions are allowed to exist.**

Every admissible transition class τ must provide one of:

### Form A — Non-increase proof (strong form)

```
∀ admissible S_t: δ_window(τ(S_t)) ≤ δ_window(S_t)
```

This is the ideal form. It means τ never increases the rolling-window
excursion of the convergence potential. Epoch advancement is preserved.

### Form B — Bounded perturbation proof (acceptable form)

```
∀ admissible S_t: δ_window(τ(S_t)) ≤ δ_window(S_t) + ε_τ
```

where `ε_τ` is a **transition-class-specific perturbation budget** satisfying:

```
ε_τ ∈ [0, ε_honest / 2]   where ε_honest = 2_000  (proof target)

and for any epoch I_t = {τ₁, ..., τₘ}:
  Σᵢ ε_τᵢ ≤ ε_honest
```

Two distinct thresholds (defined in `01_consensus.md §5`):

| Constant | Value | Role |
|----------|-------|------|
| `ε_honest` | 2_000 | Proof target. Per-epoch budget cap for §A8 Form B. TX-k budgets must sum to ≤ ε_honest. |
| `ε_halt`   | 20_000 | Halt trigger. δ_window > ε_halt triggers absorbing halt. |
| ratio      | 10×   | Safety margin: ten epochs of full-budget perturbation before halt. |

The epoch-level budget constraint (`Σᵢ ε_τᵢ ≤ ε_honest`) ensures that even if
every transaction in an epoch contributes its maximum perturbation, the total
stays within the proof target ε_honest. The 10× margin to ε_halt absorbs
unforeseen drift, accumulated round-off, or proof-discovered slack.

This prevents budget exhaustion from transaction accumulation and keeps the
halt trigger structurally unreachable in honest operation.

### Form C — Φ_safety-only effect (special case)

```
τ affects only Σ_i,t (the slash accumulator)
and does not affect D_i,t or C_i,t
```

Since `V_convergence` is defined over `D_i,t` and `C_i,t` only (not `Σ_i,t`),
a transition that exclusively modifies `Σ_i,t` trivially satisfies §A8
with `ε_τ = 0`. It must still satisfy §A8's Φ_safety monotonicity:

```
Φ_safety(τ(S_t)) ≥ Φ_safety(S_t)
```

which follows from TH-4 for any τ that only increases `Σ_i,t`.

### No other forms are admissible

A transaction type that cannot be proved to satisfy Form A, Form B, or
Form C is inadmissible. Intuition, plausibility, or performance requirements
are not sufficient grounds for admission.

**Proof obligation:** Every transaction type in `03_transactions.md` must
state and prove its §A8 form explicitly as a named lemma in
`proofs/contractivity/lyapunov_stability.v` or a dedicated file.

### Connection to TH-3

TH-3 (`δ_window(T(S_t, I_t)) ≤ ε_honest`) is proved by composing the §A8
obligations of all admitted transaction types in `I_t`:

```
TH-3 proof structure:
  ∀ τᵢ ∈ I_t: δ_window(τᵢ(S)) ≤ δ_window(S) + ε_τᵢ        (by §A8 of each τᵢ)
  Σ ε_τᵢ ≤ ε_honest                                       (epoch budget, §A8)
  ∴ δ_window(ApplyAll(S_t, I_t)) ≤ δ_window(S_t) + ε_honest   (composition)
  ≤ ε_honest                                              (TH-3 target)
  < ε_halt                                                (10× margin; halt unreachable)
```

TH-3 is provable if and only if every admitted transaction class satisfies
§A8. The axiom is the proof strategy. The 10× margin between ε_honest and
ε_halt ensures that even adversarial-but-admissible behavior cannot trigger
absorbing halt within a single epoch — halt requires accumulation across
multiple epochs, which slash accounting (Φ_safety) attributes to specific
validators.

---

## §A9 — Halt Propagation Rule

> **∀ τ, S_t: if τ(S_t) triggers any halt condition, halt_flag is set before
> any other state mutation is committed.**

Halt conditions from `00_execution_model.md §E5` (H1–H6) must be checked
**eagerly** and **before** any partial state is written. A transition that
partially mutates state and then halts leaves the system in an indeterminate
state; this is inadmissible.

```
REQUIRED halt evaluation order for any τ:

1. Evaluate all arithmetic sub-expressions in checked mode
2. If any overflow detected: halt immediately, commit nothing
3. Evaluate §A8 δ_window check on proposed τ(S_t)
4. If δ_window or Φ_safety threshold exceeded: halt, commit nothing
5. Commit state mutation atomically
6. Update state_root
```

This rule ensures that partial state mutations never become observable.
The state machine is always in a complete, consistent state — or in ⊥.

---

## §A10 — Transition Composability

> **∀ admissible τ₁, τ₂: if τ₁ ∘ τ₂ is applied in canonical order,
> the composition satisfies §A1–§A9.**

`ApplyAll(S_t, I_t)` composes multiple transitions. For the composition to
be admissible, the following must hold:

```
Composability conditions:

C1. Canonical ordering: I_t is totally ordered (no ties, no ambiguity)
C2. No mutation aliasing: if τᵢ and τⱼ share a mutation footprint field,
    their composition semantics are explicitly defined (last-write wins
    within an epoch, unless the field is subject to accumulation semantics)
C3. Budget composition: Σᵢ ε_τᵢ ≤ ε  (epoch-level §A8 budget)
C4. Encoding composability: ApplyAll(S_t, I_t) must satisfy StateWF
```

### Canonical ordering law

The canonical ordering of `I_t` is defined as:

```
Order by: H_domain(ENTROPY_ADVANCE, S_t.entropy_seed ∥ Encode(τ))

That is: for each transaction τ, compute a deterministic sort key by
hashing the current entropy seed concatenated with the encoded transaction.
Sort ascending. This produces a PRF-derived ordering that is:
  - deterministic given S_t,
  - validator-bias-resistant (no validator controls the seed),
  - replay-invariant (follows from entropy_seed determinism).
```

**This ordering law is the sole admissible transaction ordering.** Any
implementation that uses a different ordering is non-conforming.

---

## §A11 — Minimal Admissible Transaction Set (Initial)

Before the transaction algebra is expanded, the following are the only
transaction types with active proof obligations:

```
TX-0  NoOp
      mutation_footprint: ∅ (empty — no fields modified)
      §A8 form: A (trivially δ_window(τ(S_t)) = δ_window(S_t))
      proof: trivial by identity
      status: ADMITTED

TX-1  BoundedValidatorScoreDecrement
      mutation_footprint: {validators[i].score}
      §A8 form: A (score decrease reduces D_i,t, reducing V_convergence)
      proof obligation: proofs/contractivity/tx1_score_decrement.v
      status: FORMAL
```

**No other transaction types exist until their §A8 proof obligations are filed
and discharged.** This is not a temporary constraint — it is the admission policy.

New transaction types are admitted by:
1. Filing an §A8 proof obligation issue
2. Writing the proof in `proofs/contractivity/`
3. Adding the transaction to `03_transactions.md` with proof reference

---

## Appendix — Transaction Admission Template

Every entry in `03_transactions.md` must include this block:

```
## TX-N: [Name]

**Mutation footprint:** [list of fields]
**§A8 form:** [A / B / C]
**ε_τ:** [value, if Form B]
**Proof file:** proofs/contractivity/txN_[name].v
**Proof status:** [ADMITTED / PENDING / PLACEHOLDER]

**Invariant impact:**
  - affects D_i,t:     yes/no — [explanation]
  - affects C_i,t:     yes/no — [explanation]
  - affects Σ_i,t:     yes/no — [explanation]
  - monotone:          yes/no
  - bounded:           yes/no
  - δ_window effect:   [analytical bound]

**§A1 determinism argument:** [why τ has no nondeterministic sources]
**§A2 termination bound B_τ:** [static step count]
**§A4 StateWF preservation:** [proof reference or inline argument]
```

---

*End of `docs/spec/02_transition_axioms.md`*
