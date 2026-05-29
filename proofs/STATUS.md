# Proof Status

## Theorem Status

| ID | Name | Class | File | Notes |
|----|------|-------|------|-------|
| TH-1 | Encoding injectivity | FORMAL | `contractivity/encode_injectivity.v` | Fully proved. No Admitted. |
| TH-2 | Encoding totality | FORMAL | `contractivity/encode_injectivity.v` | Trivial totality. No Admitted. |
| TH-3 | Convergence decrease | FORMAL | `contractivity/lyapunov_stability.v` + `composition/th3_system_closure.v` | TH-3a, TH-3b, TH-3c all proved, with executable-step closure over the model transition gate. No Admitted. v1.0 weights (D=400k, C=350k, S=250k). |
| TH-4 | Φ_safety monotonicity | FORMAL | `safety/absorbing_halt.v` | Proved. No Admitted. |
| TH-5 | Φ_safety boundedness | FORMAL | `safety/absorbing_halt.v` | Proved. No Admitted. |
| TH-6 | Halt correctness | FORMAL | `safety/absorbing_halt.v` | Proved. No Admitted. |
| TH-7 | Replay invariance | VERIFIED | CI (golden_replay.rs + cross-ISA) | Verified on x86_64, aarch64, riscv64gc via QEMU user-static. Identical state roots across all three ISAs. Not formally proved (ISA axiomatisation deferred). |
| TH-8 | Succession soundness | FORMAL | `safety/absorbing_halt.v` + `integration/th8_composition.v` | Fully proved. Halt-frozen state in absorbing_halt.v; uniqueness composed via AX-3 in th8_composition.v. |
| TX-0 ε_τ=0 | TX-0 zero perturbation | FORMAL | `contractivity/tx_perturbation_0.v` | Fully proved. apply_tx0 leaves V_convergence unchanged (§A8 Form A). |
| TX-1 non-increase | TX-1 BoundedValidatorScoreDecrement | FORMAL | `contractivity/tx1_score_decrement.v` | Fully proved. apply_tx1 cannot increase V_convergence (§A8 Form A, stronger: non-increasing). |

## File Map

| File | Status | Theorems |
|------|--------|----------|
| `util/list_inj.v` | Compiles | Supporting lemmas: flat_map_fixed_length, flat_map_inj_fixed, app_cancel_left |
| `concat_injective.v` | Compiles | app_length_eq_left, concat_inj_fixed_left (prefix cancellation) |
| `contractivity/lyapunov_stability.v` | Compiles | TH-3a, TH-3b, TH-3c |
| `contractivity/encode_injectivity.v` | Compiles | TH-1, TH-2, state_root_collision_resistance |
| `contractivity/tx_perturbation_0.v` | Compiles | TX-0 ε_τ=0 (§A8 Form A) |
| `contractivity/lyapunov_grace_convergence.v` | Compiles | TH-GC (grace: window not full → no halt within tolerance margin) |
| `composition/th3_system_closure.v` | Compiles | Successful composed steps stay within the TH-3 epsilon envelope; Lyapunov halts imply projected delta exceeded epsilon; non-increasing transaction effects compose. |
| `safety/absorbing_halt.v` | Compiles | TH-4, TH-5, TH-6, TH-8 (partial — halt-frozen state) |
| `integration/th8_composition.v` | Compiles | TH-8 (full — uniqueness via AX-3 composition) |
| `lyapunov_decrease.v` | Compiles | V_convergence_not_monotone, V_convergence_zero_achievable |
| `cascade/cascade_health_bounded.v` | Compiles | TH-9: CH_t ∈ [0, p], cascade health term bounded |
| `cascade/cascade_determinism.v` | Compiles | TH-11: cross-ISA determinism (CI-verified; axiomatises TH-7 delegation) |
| `cascade/cascade_collision_resistance.v` | Compiles | TH-10: AX-3 reduction (Axiom declarations, no Admitted markers) |
| `blinding/blinding_non_interference.v` | Compiles | Blinding non-interference (cascade_prf_security assumption) |
| `model/Model.v` | Compiles | th3a_no_halt_within_epsilon, th3b_halt_above_epsilon, step_halted_is_identity, run_halted_is_identity |
| `model/RefinementStatement.v` | Compiles | RT1_successful_step, RT2_halt_step, RT3_halt_absorbing_epoch, RT4_halt_absorbing_flag, rust_RT1 … rust_RT4 (via AX2_rust_refinement); see docs/refinement.md |
| `ordering/causal_ordering.v` | Compiles | CO-1 sort_key_deterministic; CO-2 epoch_sortkey_lt_irrefl/trans/total; CO-3 sort_order_deterministic; validators_agree_on_sort_key |
| `ordering/compatibility_window.v` | Compiles | CW-1 compatibility_window_bound; CW-2 version_v1_0_accepted_before_window; CW-3 version_v1_0_rejected_after_window; CW-4 version_v1_1_always_accepted; CW-5 window_closure_monotone; v1_0_rejected_all_future |
| `sharding/efb_determinism.v` | Compiles | EFB root determinism for identical inputs; epoch-bound receipt replay rejection |
| `model/Extract.v` | Compiles | Extraction surface checked by `make all`; extracted OCaml is redirected to `/tmp/qash-model-extracted.ml` during proof builds. |
| `privacy/cascade_avalanche_property.v` | Compiles (Axiom) | `cascade_avalanche_property` placeholder; deferred to Domain B blinding spec / SSProve ROM formalisation |
| `privacy/oblivious_access_non_interference.v` | Compiles (Axiom) | `oblivious_access_non_interference` placeholder; deferred to blinding_params definition and Domain B blinding spec |
| `privacy/receipt_proof_soundness.v` | Compiles (Axiom) | `receipt_proof_soundness` placeholder; deferred to `06_receipts.md` and Plonky3 FRI-STARK integration |
| `privacy/blinding_health_metric.v` | Compiles (Axiom) | `blinding_health_bounded`, `blinding_halt_monotone` placeholders; deferred to §P8 metric definition in Domain B blinding spec |
| `_wip/absorbing_halt.v.draft` | Archived draft | Superseded by `safety/absorbing_halt.v` |
| `_wip/encode_injectivity.v.draft` | Archived draft | Superseded by `contractivity/encode_injectivity.v` |

## Genesis Lock Requirement

All checked theorems must compile with `coqc` (no `Admitted`) before
`GENESIS_CONSTANTS.toml` is locked. Current proof status: **PRE-GENESIS
EVIDENCE READY** for the checked proof set. Genesis lock remains intentionally
deferred until the non-proof release gates are complete: traceability artifact
reconciliation, normative PDF finalization, cross-ISA replay evidence review,
and production PAL/network readiness decisions.

## Axiom Trust Hierarchy

| Axiom | Class | Justification |
|-------|-------|---------------|
| AX-1: ISA two's complement | Implicit in Coq Z | Coq's Z is mathematical; the ISA axiom is discharged by the RISC-V/x86 formal ISA specs and the cross-compilation CI tests. |
| AX-2: Compiler correctness | Implicit in Coq extraction | Standard assumption for all verified software systems. |
| AX-3: SHA3-256 collision resistance | COMPUTATIONAL ASSUMPTION | Not mathematically provable; justified by NIST standardization and public cryptanalysis. Any audit must verify this independently. |

## Changes from Previous Draft State

The `_wip/` drafts had two categories of defects:

1. **Invalid Coq syntax** — `apply ... by X by Y` is not standard Coq.
   Fixed by rewriting all such patterns as `destruct (lemma proof1 proof2 H) as [A B]`
   or `apply (lemma proof1 proof2) in H`.

2. **Deprecated tactics** — `omega` replaced with `lia` throughout.
   Invalid lemma name `Z.gtb_ltb` replaced by flipping `sigma_update`
   to use `Z.ltb` (`INT_MAX <? sum`) and using `Z.ltb_ge` / `Z.ltb_lt`.

3. **Missing precondition** — `sigma_update_monotone` required `current ≤ INT_MAX`
   as a precondition (the admissibility invariant). Added and propagated to callers.

4. **Stray `End.`** — Bare `End.` without a matching Section/Module name
   removed from `list_inj.v` and the absorbing_halt draft.

5. **Extract Constant placement** — Moved outside the Section in
   `encode_injectivity.v` so extraction targets the globally qualified name.
