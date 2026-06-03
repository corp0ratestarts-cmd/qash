# Pure QASH Proof Status

All theorems below start as `TARGET` (Admitted stubs). No theorem is marked `PROVED`
until CI compiles actual proof content. The distinction:

| Status | Meaning |
|--------|---------|
| `PROVED` | CI compiles proof with zero non-axiom Admitted |
| `AXIOM` | Deliberately accepted as an axiom; rationale documented |
| `TARGET` | Admitted stub; proof path identified but not yet written |
| `MISSING` | No proof file exists yet |

---

## Economics (TH-E series)

| Theorem | File | Status | Notes |
|---------|------|--------|-------|
| TH-E1 Supply Delta Determinism | `proofs/economics/scarcity_axiom.v` | TARGET | |
| TH-E2 Mint Confinement | `proofs/economics/scarcity_axiom.v` | TARGET | |
| TH-E3 Reward Monotonicity | `proofs/economics/scarcity_axiom.v` | TARGET | Needs epoch_reward formalization |
| TH-E4 Tail Boundedness | `proofs/economics/scarcity_axiom.v` | TARGET | |
| TH-E5 Burn Irreversibility | `proofs/economics/scarcity_axiom.v` | **PROVED** | Trivial arithmetic |
| TH-E6 Supply Arithmetic Safety | `proofs/economics/scarcity_axiom.v` | TARGET | |
| TH-E7 Oracle Non-Interference | `proofs/economics/scarcity_axiom.v` | TARGET | |
| TH-E8 Parameter Immutability | `proofs/economics/scarcity_axiom.v` | TARGET | |
| TH-E9 Fee Ordering Non-Interference | `proofs/economics/scarcity_axiom.v` | TARGET | |
| TH-E10 Economic Commutativity | `proofs/economics/scarcity_axiom.v` | TARGET | |
| TH-E11 Conflict Annihilation | `proofs/economics/scarcity_axiom.v` | TARGET | |
| TH-E12 Signature Ordering Non-Interference | `proofs/economics/scarcity_axiom.v` | TARGET | |
| TH-E13 Inclusion Completeness | `proofs/economics/scarcity_axiom.v` | TARGET | |
| TH-E14 No Application-Layer MEV Surface | `proofs/economics/scarcity_axiom.v` | TARGET | |

## Privacy (TH-P series)

| Theorem | File | Status | Notes |
|---------|------|--------|-------|
| TH-P1 Public Graph Non-Observability | `proofs/privacy/pure_qash_non_persistence.v` | TARGET | **REQUIRED gate before genesis-candidate** |
| TH-P2 Receipt Non-Disclosure | `proofs/privacy/pure_qash_non_persistence.v` | TARGET | **REQUIRED gate before genesis-candidate** |
| TH-P3 No User Graph Persistence | `proofs/privacy/pure_qash_non_persistence.v` | TARGET | |
| TH-P4 Blind Cert Evidence Non-Disclosure | `proofs/privacy/pure_qash_non_persistence.v` | TARGET | |
| TH-P5 Regulated Profile Absence | `proofs/privacy/pure_qash_non_persistence.v` | TARGET | Enforced by CI absence guards |
