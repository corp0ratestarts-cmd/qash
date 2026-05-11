# QASH Canonical Executable Model

## Purpose

This directory contains the **canonical executable semantics** of QASH.

It is not the optimized production runtime (`crates/`).
It is not the formal proof system (`proofs/`).
It is the **bridge between them**:

```
docs/spec/   = normative law        (what the protocol IS)
proofs/      = formal theorems      (what is PROVED)
model/       ← canonical execution  (what it COMPUTES)
crates/      = production runtime   (what is DEPLOYED)
```

The model is the **ground truth** for observable behavior.
The production runtime must be observationally equivalent to it
for all admissible inputs. That equivalence is a future proof target.

---

## Design Priorities

```
1. Semantic transparency  — every state transition is explicit and auditable
2. Proof correspondence   — definitions match Coq type definitions exactly
3. Determinism visibility — no hidden caches, allocator tricks, or SIMD
4. Performance            — last priority; model is not production code
```

The model should be **executable mathematics**, not optimized software.

---

## Extraction from Coq

The preferred path for model code is **direct extraction from Coq proofs**
using Coq's extraction mechanism:

```coq
Extraction Language Haskell.   (* or OCaml *)
Extraction "model/State.hs" encode_state TH1_encode_state_injective.
```

This makes the spec/proof/model triangle trivially consistent:
the model *is* the proof, compiled. There is no separate implementation
to drift from the mathematical definitions.

Until extraction is wired up, model code is written by hand
with explicit correspondence comments linking to the Coq definitions.

---

## Invariants the Model Must Satisfy

Every function in this directory must satisfy:

```
1. Pure:        no side effects on external state
2. Total:       defined for all admissible inputs
3. Terminating: no unbounded recursion without epoch budget
4. Deterministic: same input → same output, no randomness
5. Spec-aligned: every function cites its spec section
```

---

## Spec Version

The model implements:

```
spec_hash: <computed at genesis lock>
```

This field is updated whenever `docs/spec/` changes.
The production runtime (`crates/`) must declare the same hash.

---

## Contents (Planned)

```
model/
  State.v / State.hs     — ProtocolState, ValidatorRecord definitions
  Encode.v / Encode.hs   — Encode(S_t) canonical wire format
  Transition.v           — T(S_t, I_t) reference implementation
  Stability.v            — V_convergence, Φ_safety, δ_window
  Halt.v                 — Absorbing halt semantics
  Replay.v               — Replay(G, T) reference executor
```

Files are added as the corresponding spec sections are formally closed.
No model file is written before its spec section is stable.
