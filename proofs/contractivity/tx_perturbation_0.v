(** * QASH — TX-0 Zero Perturbation (§A8 Form A obligation)

    File:    proofs/contractivity/tx_perturbation_0.v
    Spec:    docs/spec/03_transactions.md §A8, line 563
    Class:   FORMAL THEOREM

    Theorem proved
    --------------
    TX0_perturbation_zero:
      For any admissible TX-0 (a no-op nonce-advance), applying it to a state
      leaves V_convergence unchanged.  This is the ε_τ = 0 claim from §A8.

      More precisely: apply_tx0 only increments a single nonce; the
      V_convergence functional depends exclusively on divergence and conflict
      metrics (not nonces), so the functional value is invariant.

    Imports / dependencies
    ----------------------
    Uses the V_convergence model from lyapunov_stability.v (weight_D, weight_C,
    V_convergence definition, validator record).

    Axioms used
    -----------
    None beyond Coq's standard library.

    Status: Fully proved.  No Admitted markers.
*)

Require Import Coq.ZArith.ZArith.
Require Import Coq.micromega.Lia.
Open Scope Z_scope.

(* ================================================================= *)
(** ** §0 — Constants (copied from lyapunov_stability.v for self-containment) *)
(* ================================================================= *)

Definition weight_D : Z := 400_000.
Definition weight_C : Z := 350_000.
Definition scale    : Z := 1_000_000.
Definition N_max    : Z := 1024.

(* ================================================================= *)
(** ** §1 — Validator record and state models                         *)
(* ================================================================= *)

(** A single validator's consensus metrics (only the fields relevant to
    V_convergence; nonce is tracked separately). *)
Record ValidatorMetrics : Type := mkVM {
  divergence  : Z;
  conflict    : Z;
  slash_accum : Z;
}.

(** Admissibility bounds: D, C ∈ [0, scale]; Σ ≥ 0. *)
Definition vm_admissible (v : ValidatorMetrics) : Prop :=
  0 <= divergence v /\ divergence v <= scale /\
  0 <= conflict v    /\ conflict v    <= scale /\
  0 <= slash_accum v.

(** Per-validator contribution to V_convergence. *)
Definition v_term (v : ValidatorMetrics) : Z :=
  weight_D * divergence v + weight_C * conflict v.

(** Full state: a list of validator metrics plus a nonce array (modelled
    as a list of Z).  We keep metrics and nonces separate to mirror the
    Rust EpochState layout. *)
Record EpochState : Type := mkState {
  validators : list ValidatorMetrics;
  nonces     : list Z;
}.

(** V_convergence = Σ_i v_term(validators[i]). *)
Fixpoint v_convergence_list (vs : list ValidatorMetrics) : Z :=
  match vs with
  | nil     => 0
  | v :: tl => v_term v + v_convergence_list tl
  end.

Definition V_convergence (s : EpochState) : Z :=
  v_convergence_list (validators s).

(* ================================================================= *)
(** ** §2 — TX-0 application model                                    *)
(* ================================================================= *)

(** TX-0 is characterised entirely by the index of the validator whose
    nonce is incremented.  No metric field is touched. *)

(** Increment the n-th element of a list of Z values. *)
Fixpoint incr_nth (l : list Z) (n : nat) : list Z :=
  match l, n with
  | nil,     _    => nil
  | x :: tl, O   => (x + 1) :: tl
  | x :: tl, S n' => x :: incr_nth tl n'
  end.

(** apply_tx0 i s: increment nonces[i], leave validators unchanged. *)
Definition apply_tx0 (i : nat) (s : EpochState) : EpochState :=
  mkState (validators s) (incr_nth (nonces s) i).

(* ================================================================= *)
(** ** §3 — Core lemma: validators list is unchanged                   *)
(* ================================================================= *)

Lemma apply_tx0_validators_unchanged :
  forall (i : nat) (s : EpochState),
    validators (apply_tx0 i s) = validators s.
Proof.
  intros i s.
  unfold apply_tx0.
  simpl.
  reflexivity.
Qed.

(* ================================================================= *)
(** ** §4 — V_convergence is a function of validators only            *)
(* ================================================================= *)

Lemma v_convergence_depends_only_on_validators :
  forall (s1 s2 : EpochState),
    validators s1 = validators s2 ->
    V_convergence s1 = V_convergence s2.
Proof.
  intros s1 s2 Heq.
  unfold V_convergence.
  rewrite Heq.
  reflexivity.
Qed.

(* ================================================================= *)
(** ** §5 — Main theorem: TX-0 has zero perturbation (ε_τ = 0)       *)
(* ================================================================= *)

(** §A8 Form A obligation for TX-0:
    Applying an admissible TX-0 (index i) to any state leaves
    V_convergence unchanged. *)
Theorem TX0_perturbation_zero :
  forall (i : nat) (s : EpochState),
    V_convergence (apply_tx0 i s) = V_convergence s.
Proof.
  intros i s.
  apply v_convergence_depends_only_on_validators.
  apply apply_tx0_validators_unchanged.
Qed.

(** Corollary: the Lyapunov condition check is identical before and after
    a TX-0 application, so the halt gate is unaffected. *)
Corollary TX0_does_not_trigger_halt :
  forall (i : nat) (s : EpochState) (epsilon : Z),
    V_convergence s <= epsilon <->
    V_convergence (apply_tx0 i s) <= epsilon.
Proof.
  intros i s eps.
  rewrite TX0_perturbation_zero.
  tauto.
Qed.
