(** * QASH — TX-1 BoundedValidatorScoreDecrement Perturbation Bound

    File:    proofs/contractivity/tx1_score_decrement.v
    Spec:    docs/spec/02_transition_axioms.md §TX-1
             docs/spec/03_transactions.md §TX-1
    Class:   FORMAL THEOREM

    §A8 obligation
    --------------
    TX-1 (BoundedValidatorScoreDecrement) applies a bounded downward
    adjustment to one validator's divergence score D_i,t.  Because the new
    divergence cannot exceed the old one and weight_D > 0, V_convergence is
    non-increasing under TX-1.  This is strictly stronger than Form A
    (equality): the transaction can only improve convergence, never degrade it.

    Theorems proved
    ---------------
    TX1_score_decrement_nonincreasing :
      0 <= new_D <= divergence(validators[i]) ->
      V_convergence(apply_tx1 i new_D s) <= V_convergence(s)

    TX1_score_decrement_does_not_trigger_halt :
      If V_convergence(s) <= threshold then
      V_convergence(apply_tx1 i new_D s) <= threshold.

    Imports / dependencies
    ----------------------
    Self-contained; duplicates the type definitions from tx_perturbation_0.v
    to avoid a cross-file dependency on Tier-2 compile ordering.

    Axioms used
    -----------
    None beyond Coq's standard library.

    Status: Fully proved.  No Admitted markers.
*)

Require Import Coq.ZArith.ZArith.
Require Import Coq.Lists.List.
Require Import Coq.micromega.Lia.
Open Scope Z_scope.

(* ================================================================= *)
(** ** §0 — Constants (from GENESIS_CONSTANTS.toml v1.0)              *)
(* ================================================================= *)

Definition weight_D : Z := 400_000.
Definition weight_C : Z := 350_000.
Definition scale    : Z := 1_000_000.

(* ================================================================= *)
(** ** §1 — Validator record and state models                          *)
(* ================================================================= *)

Record ValidatorMetrics : Type := mkVM {
  divergence  : Z;
  conflict    : Z;
  slash_accum : Z;
}.

Definition vm_admissible (v : ValidatorMetrics) : Prop :=
  0 <= divergence v /\ divergence v <= scale /\
  0 <= conflict v    /\ conflict v    <= scale /\
  0 <= slash_accum v.

Definition v_term (v : ValidatorMetrics) : Z :=
  weight_D * divergence v + weight_C * conflict v.

Record EpochState : Type := mkState {
  validators : list ValidatorMetrics;
  nonces     : list Z;
}.

Fixpoint v_convergence_list (vs : list ValidatorMetrics) : Z :=
  match vs with
  | nil     => 0
  | v :: tl => v_term v + v_convergence_list tl
  end.

Definition V_convergence (s : EpochState) : Z :=
  v_convergence_list (validators s).

(* ================================================================= *)
(** ** §2 — TX-1 application model                                     *)
(* ================================================================= *)

(** Set the divergence field of the n-th validator to new_D.
    All other fields (conflict, slash_accum, nonces) are untouched. *)
Fixpoint set_divergence_nth
    (vs : list ValidatorMetrics) (n : nat) (new_D : Z)
    : list ValidatorMetrics :=
  match vs, n with
  | nil,     _    => nil
  | v :: tl, O   => mkVM new_D (conflict v) (slash_accum v) :: tl
  | v :: tl, S n' => v :: set_divergence_nth tl n' new_D
  end.

(** apply_tx1 i new_D s:
      - Set validators[i].divergence to new_D.
      - Leave all other validator fields and nonces unchanged.
    Admission precondition (checked by the runtime before calling):
      0 <= new_D <= divergence(validators[i]). *)
Definition apply_tx1 (i : nat) (new_D : Z) (s : EpochState) : EpochState :=
  mkState (set_divergence_nth (validators s) i new_D) (nonces s).

(* ================================================================= *)
(** ** §3 — Monotonicity of v_term under divergence substitution       *)
(* ================================================================= *)

(** Replacing a validator's divergence with a smaller value cannot
    increase v_term.  Destruct the record so projections are concrete,
    then lia closes the linear arithmetic goal. *)
Lemma v_term_set_divergence_nonincreasing :
  forall (v : ValidatorMetrics) (new_D : Z),
    new_D <= divergence v ->
    v_term (mkVM new_D (conflict v) (slash_accum v)) <= v_term v.
Proof.
  intros v new_D Hle.
  (* The projections on the concrete mkVM constructor are definitional equalities;
     reflexivity proves them, then rewrite makes the goal purely arithmetic. *)
  assert (Hdiv : divergence (mkVM new_D (conflict v) (slash_accum v)) = new_D)
    by reflexivity.
  assert (Hcon : conflict  (mkVM new_D (conflict v) (slash_accum v)) = conflict v)
    by reflexivity.
  unfold v_term.
  rewrite Hdiv, Hcon.
  unfold weight_D, weight_C.
  lia.
Qed.

(* ================================================================= *)
(** ** §4 — List-level non-increase under set_divergence_nth           *)
(* ================================================================= *)

(** Applying set_divergence_nth with new_D <= old_D cannot increase
    v_convergence_list.  Structural induction on (vs, i). *)
Lemma v_convergence_list_set_divergence_nonincreasing :
  forall (vs : list ValidatorMetrics) (i : nat) (new_D : Z),
    new_D <= (match nth_error vs i with
              | Some v => divergence v
              | None   => new_D     (* out-of-range: no-op, trivially ≤ *)
              end) ->
    v_convergence_list (set_divergence_nth vs i new_D) <=
    v_convergence_list vs.
Proof.
  induction vs as [| v tl IH]; intros i new_D Hle.
  - (* nil: both sides are 0 *)
    simpl. lia.
  - destruct i as [| i'].
    + (* i = 0: head divergence decreases; tail is unchanged *)
      simpl in Hle.
      simpl.
      apply Z.add_le_mono_r.
      apply v_term_set_divergence_nonincreasing.
      exact Hle.
    + (* i = S i': head is untouched; apply IH to tail *)
      simpl.
      apply Z.add_le_mono_l.
      apply IH.
      simpl in Hle.
      exact Hle.
Qed.

(* ================================================================= *)
(** ** §5 — Main theorem: TX-1 is V_convergence non-increasing         *)
(* ================================================================= *)

(** §A8 Form A obligation for TX-1 (stronger than equality — the
    transaction provably cannot increase V_convergence): *)
Theorem TX1_score_decrement_nonincreasing :
  forall (i : nat) (new_D : Z) (s : EpochState),
    new_D <= (match nth_error (validators s) i with
              | Some v => divergence v
              | None   => new_D
              end) ->
    V_convergence (apply_tx1 i new_D s) <= V_convergence s.
Proof.
  intros i new_D s Hle.
  unfold V_convergence, apply_tx1.
  simpl.
  apply v_convergence_list_set_divergence_nonincreasing.
  exact Hle.
Qed.

(** Corollary: if the halt threshold was not breached before TX-1,
    it cannot be breached by TX-1 alone. *)
Corollary TX1_score_decrement_does_not_trigger_halt :
  forall (i : nat) (new_D : Z) (s : EpochState) (threshold : Z),
    new_D <= (match nth_error (validators s) i with
              | Some v => divergence v
              | None   => new_D
              end) ->
    V_convergence s <= threshold ->
    V_convergence (apply_tx1 i new_D s) <= threshold.
Proof.
  intros i new_D s threshold Hle Hthresh.
  apply Z.le_trans with (V_convergence s).
  - apply TX1_score_decrement_nonincreasing. exact Hle.
  - exact Hthresh.
Qed.
