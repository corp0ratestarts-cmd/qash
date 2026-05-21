# Theorem 3 System-Closure Plan

This plan captures the remaining work to move TH-3 from a local proof subset
(TH-3a/TH-3b/TH-3c) to full system-level convergence certainty.

## Current verified subset

`proofs/contractivity/lyapunov_stability.v` currently discharges:

- TH-3a: `δ_window ≤ ε -> halt_triggered = false`
- TH-3b: `halt_triggered <-> δ_window > ε`
- TH-3c: `FinalizeEpoch -> V_convergence = 0`

These results are necessary but not sufficient for full protocol convergence.

## Remaining closure objectives (in order)

## 1) End-to-end replay invariance composition

Goal: prove TH-3 properties are preserved across the full transition pipeline,
not only local Lyapunov arithmetic.

- [x] Connect `docs/spec/02_transition_axioms.md` §A8 obligations to executable transition composition.
- [x] Prove admitted transaction classes preserve the TH-3 envelope at the current TX-0/TX-1 surface.
- [x] Add proof artifact that composes per-transaction bounds into global `ε_honest` bound.

Proof artifact: `proofs/composition/th3_system_closure.v`.

## 2) Proof-to-runtime correspondence

Goal: show the Coq model and runtime implementation compute equivalent TH-3-relevant state deltas.

- [x] Define a trace projection for runtime events used by `V_convergence` and `δ_window`.
- [x] Add vector fixtures that compare model-vs-runtime outputs for TH-3-critical transitions.
- [x] Gate CI on correspondence checks for the projection (proof artifact + executable fixture run).

## 3) Adversarial replay corpus

Goal: demonstrate operational determinism under hostile or pathological replay inputs.

- [x] Build deterministic malformed-sequence corpus (invalid successor attempts, bound-edge deltas, conflicting admission order).
- [x] Run corpus across cross-ISA matrix and require identical reject/accept + state-root outcomes.
- [x] Track corpus IDs in errata/ADR references for reproducible failure triage.

Corpus IDs:

- `ARC-DECODE-INVALID`: `adversarial_replay_corpus_decode_invalid_is_deterministic`
- `ARC-EPSILON-BOUNDARY`: `adversarial_replay_corpus_epsilon_boundary_is_accepted`
- `ARC-TX1-ORDER`: `adversarial_replay_corpus_tx1_valid_and_invalid_paths_are_deterministic`

## Exit criteria for TH-3 “CLOSED”

TH-3 may be promoted to CLOSED only when:

1. TH-3a/3b/3c compile under `coqc` on pinned toolchain.
2. Composition theorem from transition axioms to the current admissible transition set is discharged.
3. Model/runtime correspondence checks pass in CI for TH-3 projection vectors.
4. Adversarial replay corpus passes cross-ISA determinism checks.
5. `proofs/STATUS.md` and project status tables are updated in the same PR.
