(** * QASH — TX-0 §A8 Perturbation Bound

    File:    proofs/contractivity/tx_perturbation_0.v
    Spec:    docs/spec/03_transactions.md §TX-0; docs/spec/02_transition_axioms.md §A8
    Class:   FORMAL THEOREM

    TX-0 changes only the author's nonce. The convergence potential
    V_convergence depends only on divergence and conflict metrics, so TX-0
    preserves V_convergence exactly and satisfies §A8 Form A with epsilon = 0.
*)

Require Import Coq.ZArith.ZArith.
Require Import Coq.micromega.Lia.
Open Scope Z_scope.

Definition weight_D : Z := 400000.
Definition weight_C : Z := 350000.

Record Tx0ValidatorMetrics : Type := mkTx0VM {
  tx0_divergence : Z;
  tx0_conflict : Z;
  tx0_nonce : Z
}.

Definition tx0_v_convergence (v : Tx0ValidatorMetrics) : Z :=
  weight_D * tx0_divergence v + weight_C * tx0_conflict v.

Definition tx0_apply_nonce_update
    (v : Tx0ValidatorMetrics)
    (nonce_next : Z) : Tx0ValidatorMetrics :=
  mkTx0VM (tx0_divergence v) (tx0_conflict v) nonce_next.

(** TX-0 preserves the convergence potential exactly. *)
Theorem TX0_v_convergence_invariant :
  forall v nonce_next,
    tx0_v_convergence (tx0_apply_nonce_update v nonce_next) =
    tx0_v_convergence v.
Proof.
  intros v nonce_next.
  unfold tx0_v_convergence, tx0_apply_nonce_update.
  reflexivity.
Qed.

(** §A8 Form A, stated over the delta-window expression.

    The window minimum is an arbitrary value here because TX-0 does not modify
    either the window or the fields used by V_convergence. *)
Theorem TX0_perturbation_bound :
  forall v nonce_next window_min,
    tx0_v_convergence (tx0_apply_nonce_update v nonce_next) - window_min <=
    tx0_v_convergence v - window_min.
Proof.
  intros v nonce_next window_min.
  rewrite TX0_v_convergence_invariant.
  lia.
Qed.

(** Stronger equality form useful when composing TH-3 proper. *)
Corollary TX0_delta_window_invariant :
  forall v nonce_next window_min,
    tx0_v_convergence (tx0_apply_nonce_update v nonce_next) - window_min =
    tx0_v_convergence v - window_min.
Proof.
  intros v nonce_next window_min.
  rewrite TX0_v_convergence_invariant.
  reflexivity.
Qed.

(** Status: closed, no Admitted markers. *)
