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

- [ ] Connect `docs/spec/02_transition_axioms.md` §A8 obligations to executable transition composition.
- [ ] Prove each admitted transaction class preserves the TH-3 envelope under canonical encoding.
- [ ] Add proof artifact that composes per-transaction bounds into global `ε_honest` bound.

## 2) Proof-to-runtime correspondence

Goal: show the Coq model and runtime implementation compute equivalent TH-3-relevant state deltas.

- [ ] Define a trace projection for runtime events used by `V_convergence` and `δ_window`.
- [ ] Add vector fixtures that compare model-vs-runtime outputs for TH-3-critical transitions.
- [ ] Gate CI on correspondence checks for the projection (proof artifact + executable fixture run).

## 3) Adversarial replay corpus

Goal: demonstrate operational determinism under hostile or pathological replay inputs.

- [ ] Build deterministic malformed-sequence corpus (invalid successor attempts, bound-edge deltas, conflicting admission order).
- [ ] Run corpus across cross-ISA matrix and require identical reject/accept + state-root outcomes.
- [ ] Track corpus IDs in errata/ADR references for reproducible failure triage.

## Exit criteria for TH-3 “CLOSED”

TH-3 may be promoted to CLOSED only when:

1. TH-3a/3b/3c compile under `coqc` on pinned toolchain.
2. Composition theorem from transition axioms to full admissible transition set is discharged.
3. Model/runtime correspondence checks pass in CI for TH-3 projection vectors.
4. Adversarial replay corpus passes cross-ISA determinism checks.
5. `proofs/STATUS.md` and README status tables are updated in the same PR.
