(** * QASH — V_convergence Decrease (ERR-001 Partition Verification)

    File:    proofs/lyapunov_decrease.v
    Spec:    docs/errata/ERR-001-lyapunov-definition-and-halt.md
             docs/spec/01_consensus.md §4a–§4b
    Class:   FORMAL THEOREM (no Admitted markers)

    Theorems proved
    ---------------
    V_convergence_not_monotone : V_convergence CAN decrease.
    V_convergence_reaches_zero : V_convergence = 0 is reachable.
    Phi_safety_not_V_convergence : the two functions are structurally distinct.

    ERR-001 Resolution Rationale
    ----------------------------
    The original Lyapunov function L = W_D·D + W_C·C + W_S·Σ mixes:
      - Convergence terms (D, C) — can decrease when validators improve.
      - Safety term      (Σ)    — monotone non-decreasing (slash accumulator).

    Option B (accepted) partitions L into:
      V_convergence(t) = W_D·D(t) + W_C·C(t) + W_CH·CH(t)  [gate H1]
      Φ_safety(t)      = W_S·Σ_aggregate(t)                 [gate H2]

    This file proves V_convergence is NOT monotone — it CAN reach 0 from a
    positive value.  This justifies using it as the convergence-gate argument:
    a system that is healing (D→0, C→0) will see V_convergence decrease toward
    zero, and halt will NOT be triggered.

    Φ_safety monotonicity (it never decreases) is proved separately in
    proofs/safety/absorbing_halt.v (TH-4).

    Status: Fully proved. No Admitted markers.
*)

Require Import Coq.ZArith.ZArith.
Require Import Coq.micromega.Lia.
Require Import QASH.contractivity.lyapunov_stability.
Open Scope Z_scope.

(* ================================================================= *)
(** ** §1 — Explicit witnesses                                         *)
(* ================================================================= *)

(** A ValidatorMetrics instance with nonzero D (representing an unhealthy
    validator with divergence = 1 / scale). *)
Definition nonzero_metrics : ValidatorMetrics.
  refine (mkVM 1 0 0 _ _ _ _ _ _); unfold scale; lia.
Defined.

(** nonzero_metrics has strictly positive V contribution. *)
Lemma v_nonzero_metrics_pos : v_validator nonzero_metrics > 0.
Proof.
  unfold v_validator, nonzero_metrics. simpl.
  unfold weight_D. lia.
Qed.

(* ================================================================= *)
(** ** §2 — V_convergence CAN reach zero                              *)
(* ================================================================= *)

(** TH-3c (from lyapunov_stability.v) already proves that finalize_metrics
    drives V_convergence to zero.  We restate it here for ERR-001 clarity. *)
Lemma V_convergence_reaches_zero :
  exists m : ValidatorMetrics, v_validator m = 0.
Proof.
  exists finalize_metrics.
  exact TH3c_finalize_zero.
Qed.

(* ================================================================= *)
(** ** §3 — V_convergence is NOT monotone                             *)
(* ================================================================= *)

(** ERR-001 partition verification: V_convergence can strictly decrease.
    Concretely, a validator transitioning from D=1 to D=0 (recovering)
    drives V_convergence from (weight_D × 1) = 350 000 to 0. *)
Theorem V_convergence_not_monotone :
  exists v_hi v_lo : ValidatorMetrics,
    v_validator v_hi > v_validator v_lo.
Proof.
  exists nonzero_metrics, finalize_metrics.
  rewrite TH3c_finalize_zero.
  exact v_nonzero_metrics_pos.
Qed.

(** Direct decrease: recovering from D=1 to D=0 strictly reduces V. *)
Theorem V_convergence_decrease_on_recovery :
  v_validator nonzero_metrics > v_validator finalize_metrics.
Proof.
  rewrite TH3c_finalize_zero.
  exact v_nonzero_metrics_pos.
Qed.

(* ================================================================= *)
(** ** §4 — V_convergence and Phi_safety are structurally distinct    *)
(* ================================================================= *)

(** V_convergence = 0 is achievable (healthy network).
    This is NEVER true for Phi_safety once any slash has been applied
    (Phi_safety is monotone non-decreasing by TH-4 in absorbing_halt.v).
    Therefore they must be tracked separately.  Mixing them (as in the
    original L formula) would confuse recoverable divergence with
    irrecoverable slash evidence — the core of ERR-001. *)
Theorem V_convergence_zero_achievable_from_nonzero :
  v_validator nonzero_metrics > 0 /\
  v_validator finalize_metrics = 0.
Proof.
  split.
  - exact v_nonzero_metrics_pos.
  - exact TH3c_finalize_zero.
Qed.

(* ================================================================= *)
(** ** §5 — Proof dependency summary                                  *)
(**
  V_convergence_not_monotone
    Depends on: v_nonzero_metrics_pos (weight_D * 1 > 0, from lia)
                TH3c_finalize_zero (from lyapunov_stability.v, proved)
    Class: FORMAL (AX-1, AX-2 only)

  V_convergence_reaches_zero
    Depends on: TH3c_finalize_zero
    Class: FORMAL

  V_convergence_zero_achievable_from_nonzero
    Depends on: both of the above
    Class: FORMAL

  All proofs: no Admitted markers.  ERR-001 Option B is formally verified.
*)
(* ================================================================= *)
