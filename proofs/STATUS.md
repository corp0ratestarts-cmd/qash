# Proof Status

This file is the repository source of truth for proof readiness at genesis
lock. A theorem is **closed** only when it is in an active `*.v` file outside
`proofs/_wip/`, contains no `Admitted`/`admit` marker, and compiles with
`coqc` in the `coq-proofs` CI job.

Files in `proofs/_wip/` are design drafts. They may contain complete proof
strategies or no-`Admitted` comments, but they are not genesis-lock evidence
until migrated to active `*.v` files and compiled by CI.

## Active proof inventory

| File | Scope | CI gate |
|------|-------|---------|
| `proofs/contractivity/lyapunov_stability.v` | TH-3a, TH-3b, TH-3c foundation lemmas | `coq-proofs` |
| `proofs/contractivity/tx_perturbation_0.v` | TX-0 §A8 Form A perturbation bound | `coq-proofs` |
| `proofs/util/list_inj.v` | List/encoding utility lemmas | `coq-proofs` |

## Theorem Status

| ID | Name | Class | Status | Notes |
|----|------|-------|--------|-------|
| TH-1 | Encoding injectivity | FORMAL | 🟡 DRAFT | `_wip/encode_injectivity.v.draft` claims the final proof shape and no `Admitted`, but it is not an active `coqc`-checked file yet. |
| TH-2 | Encoding totality | FORMAL | 🟡 DRAFT | Same draft as TH-1; not an active genesis-lock proof yet. |
| TH-3 | Convergence decrease | FORMAL | 🟡 PARTIAL | TH-3a/TH-3b halt-gate lemmas and TH-3c finalize-zero are active; TH-3 proper still needs §A8 composition across admitted transactions. |
| TX-0 §A8 | No-op perturbation bound | FORMAL | ✅ ACTIVE | `TX0_perturbation_bound` proves Form A for the only admitted no-op transaction model. |
| TH-4 | Φ_safety monotonicity | FORMAL | 🟡 DRAFT | `_wip/absorbing_halt.v.draft`; not active or CI-checked. |
| TH-5 | Φ_safety boundedness | FORMAL | 🟡 DRAFT | `_wip/absorbing_halt.v.draft`; not active or CI-checked. |
| TH-6 | Halt correctness | FORMAL | 🟡 DRAFT | `_wip/absorbing_halt.v.draft`; not active or CI-checked. |
| TH-7 | Replay invariance | VERIFIED | 🟡 PARTIAL | CI-tested at the Rust level; full test-vector suite and/or formal replay proof remains open. |
| TH-8 | Succession soundness | FORMAL | 🟡 PARTIAL/DRAFT | Draft depends on a placeholder `state_root_injective_import`; must be wired to the TH-1/AX-3 state-root collision-resistance lemma before closure. |

## Current blockers

1. Migrate TH-1/TH-2 out of `_wip/` and make them compile under `coq-proofs`.
2. Finish TH-3 proper by composing every admitted transaction's §A8 obligation
   against the per-epoch `ε_honest` budget.
3. Migrate TH-4/TH-5/TH-6 from `_wip/absorbing_halt.v.draft` to active Coq,
   replacing obsolete tactics and checking the stated preconditions.
4. Replace TH-8's `state_root_injective_import` placeholder with the real
   state-root collision-resistance corollary derived from TH-1 plus AX-3.
5. Expand TH-7 from partial CI checks to the full replay/test-vector gate.

## Genesis lock requirement

Before `GENESIS_CONSTANTS.toml` is locked, all genesis-lock theorems must be in
active `*.v` files, contain no `Admitted`/`admit`, and pass `coq-proofs` in CI.
Draft files and README claims do not satisfy this gate.
