(** * QASH — Executable Protocol Model (Coq ↔ Rust bridge)

    File:    proofs/model/Model.v
    Issue:   #19 — Practical Coq ↔ Rust Integration

    This module defines the QASH state machine as a Coq-executable function.
    It serves as the shared reference between formal proofs and the Rust
    implementation, enabling:

      1. Extraction of test vectors (Option A from issue #19)
      2. Direct linkage to existing TH-3a/TH-3b/TH-6 proofs
      3. CI parity: Rust tests in coq_vectors.rs load proofs/model/vectors.json
         and assert that advance_epoch produces identical outputs

    Relationship to existing proofs:
      TH-3a/3b: lyapunov_stability.v — proved using the same constants below
      TH-6:     absorbing_halt.v    — step on halted state is identity
      TH-8:     integration/th8_composition.v — composition invariant

    Status: FORMAL — compiles with coqc -Q . QASH. No Admitted markers.
*)

Require Import Coq.ZArith.ZArith.
Require Import Coq.Bool.Bool.
Require Import Coq.Lists.List.
Require Import Coq.micromega.Lia.
Open Scope Z_scope.

(* ================================================================= *)
(** ** §0 — Protocol Constants (v1.1, matching GENESIS_CONSTANTS.toml) *)
(* ================================================================= *)

Definition weight_D  : Z := 350_000.
Definition weight_C  : Z := 300_000.
Definition weight_S  : Z := 200_000.
Definition epsilon   : Z :=  20_000.
Definition scale     : Z := 1_000_000.
Definition window_sz : nat := 3.

(* ================================================================= *)
(** ** §1 — State Types                                               *)
(* ================================================================= *)

(** Per-validator metrics. All values are FixedPoint integers ∈ [0, scale]. *)
Record ValidatorMetrics := mkVM {
  vm_D  : Z;   (** Divergence  D_i ∈ [0, scale] *)
  vm_C  : Z;   (** Conflict    C_i ∈ [0, scale] *)
  vm_S  : Z;   (** Slash accum Σ_i ≥ 0          *)
}.

Definition zero_vm : ValidatorMetrics := mkVM 0 0 0.

(** V(v) = α·D + β·C per validator. Matches lyapunov.rs evaluate(). *)
Definition v_validator (v : ValidatorMetrics) : Z :=
  weight_D * vm_D v + weight_C * vm_C v.

(** Sum V over a list of validators. *)
Fixpoint v_sum (vs : list ValidatorMetrics) : Z :=
  match vs with
  | nil      => 0
  | v :: rest => v_validator v + v_sum rest
  end.

(** Halt reasons (mirror HaltReason enum in Rust, §A6). *)
Inductive HaltReason : Type :=
  | HR_None            (** 0x00 — running *)
  | HR_LyapunovViolation  (** 0x01 *)
  | HR_DecodeInvalid.  (** 0x04 *)

Definition is_halted (h : HaltReason) : bool :=
  match h with
  | HR_None => false
  | _       => true
  end.

(** Convergence window: a bounded circular buffer of V values. *)
Record ConvWindow := mkCW {
  cw_values : list Z; (** at most window_sz entries, newest first *)
}.

Definition empty_window : ConvWindow := mkCW nil.

Definition cw_push (w : ConvWindow) (v : Z) : ConvWindow :=
  let vs := cw_values w in
  let new_vs := v :: vs in
  if Nat.leb (List.length new_vs) window_sz
  then mkCW new_vs
  else mkCW (List.firstn window_sz new_vs).

Definition cw_is_full (w : ConvWindow) : bool :=
  Nat.eqb (List.length (cw_values w)) window_sz.

Fixpoint list_min (xs : list Z) (default : Z) : Z :=
  match xs with
  | nil      => default
  | x :: nil => x
  | x :: rest => Z.min x (list_min rest default)
  end.

Definition cw_min (w : ConvWindow) : Z :=
  list_min (cw_values w) 0.

(** delta_window = V_current - min(window). TH-3b: halt iff delta > epsilon. *)
Definition delta_window (w : ConvWindow) (v_current : Z) : Z :=
  if cw_is_full w
  then v_current - cw_min w
  else 0.

(** Minimal protocol state for the model. *)
Record ModelState := mkMS {
  ms_epoch        : Z;
  ms_halt         : HaltReason;
  ms_validators   : list ValidatorMetrics;
  ms_window       : ConvWindow;
}.

(** Per-validator update. None = idle (keep existing metrics). *)
Inductive ValidatorUpdate : Type :=
  | VU_Idle
  | VU_Update (d c s : Z).

Definition apply_update (v : ValidatorMetrics) (u : ValidatorUpdate)
    : option ValidatorMetrics :=
  match u with
  | VU_Idle => Some v
  | VU_Update d c s =>
    (* Validate bounds — §A4: D, C ∈ [0, scale]; S monotone non-decreasing. *)
    if andb (andb (Z.leb 0 d) (Z.leb d scale))
            (andb (andb (Z.leb 0 c) (Z.leb c scale))
                  (andb (Z.leb 0 s) (Z.leb (vm_S v) s)))
    then Some (mkVM d c s)
    else None  (* DecodeInvalid *)
  end.

Fixpoint apply_updates (vs : list ValidatorMetrics)
                       (us : list ValidatorUpdate)
    : option (list ValidatorMetrics) :=
  match vs, us with
  | nil, nil             => Some nil
  | nil, _               => None
  | _, nil               => None
  | v :: vs', u :: us'   =>
    match apply_update v u, apply_updates vs' us' with
    | Some v', Some rest => Some (v' :: rest)
    | _, _               => None
    end
  end.

(** The core epoch transition function.
    This is the Coq executable model corresponding to advance_epoch() in Rust.
    Mirrors: crates/consensus/src/transition.rs::advance_epoch
*)
Definition step (s : ModelState) (updates : list ValidatorUpdate)
    : ModelState :=
  (* §A6: halted state is absorbing — return unchanged. *)
  if is_halted (ms_halt s) then s
  else
    (* §A4: validate and apply updates. *)
    match apply_updates (ms_validators s) updates with
    | None =>
        mkMS (ms_epoch s) HR_DecodeInvalid (ms_validators s) (ms_window s)
    | Some new_vs =>
        let v_cur := v_sum new_vs in
        let delta  := delta_window (ms_window s) v_cur in
        (* §TH-3b: halt iff delta > epsilon. *)
        if Z.ltb epsilon delta
        then mkMS (ms_epoch s) HR_LyapunovViolation (ms_validators s) (ms_window s)
        else
          let new_window := cw_push (ms_window s) v_cur in
          mkMS (ms_epoch s + 1) HR_None new_vs new_window
    end.

(** Apply a sequence of updates (iterated step). *)
Fixpoint run (s : ModelState) (inputs : list (list ValidatorUpdate))
    : ModelState :=
  match inputs with
  | nil         => s
  | us :: rest  =>
    let s' := step s us in
    if is_halted (ms_halt s')
    then s'
    else run s' rest
  end.

(* ================================================================= *)
(** ** §2 — Lemmas: Halt Is Absorbing (TH-6 analogue)                *)
(* ================================================================= *)

Lemma step_halted_is_identity :
  forall s us,
    is_halted (ms_halt s) = true ->
    step s us = s.
Proof.
  intros s us Hh.
  unfold step. rewrite Hh. reflexivity.
Qed.

Lemma run_halted_is_identity :
  forall inputs s,
    is_halted (ms_halt s) = true ->
    run s inputs = s.
Proof.
  induction inputs as [| us rest IH]; intros s Hh.
  - reflexivity.
  - simpl. rewrite step_halted_is_identity; [| exact Hh].
    rewrite Hh. reflexivity.
Qed.

(* ================================================================= *)
(** ** §3 — Lemma: epoch monotone on non-halt step                    *)
(* ================================================================= *)

Lemma step_epoch_advance :
  forall s us s',
    s' = step s us ->
    is_halted (ms_halt s) = false ->
    ms_halt s' = HR_None ->
    ms_epoch s' = ms_epoch s + 1.
Proof.
  intros s us s' Hstep Hnh Hok.
  unfold step in Hstep.
  rewrite Hnh in Hstep.
  destruct (apply_updates (ms_validators s) us) as [vs|] eqn:Hau.
  - destruct (Z.ltb epsilon (delta_window (ms_window s) (v_sum vs))) eqn:Hdelta.
    + subst. simpl in Hok. discriminate.
    + subst. simpl. lia.
  - subst. simpl in Hok. discriminate.
Qed.

(* ================================================================= *)
(** ** §4 — Theorem: No-halt when delta ≤ epsilon (TH-3a)            *)
(* ================================================================= *)

Theorem th3a_no_halt_within_epsilon :
  forall s us,
    is_halted (ms_halt s) = false ->
    apply_updates (ms_validators s) us <> None ->
    (forall vs,
       apply_updates (ms_validators s) us = Some vs ->
       delta_window (ms_window s) (v_sum vs) <= epsilon) ->
    ms_halt (step s us) = HR_None.
Proof.
  intros s us Hnh Hsome Hdelta.
  unfold step. rewrite Hnh.
  destruct (apply_updates (ms_validators s) us) as [vs|] eqn:Hau.
  - specialize (Hdelta vs eq_refl).
    assert (Z.ltb epsilon (delta_window (ms_window s) (v_sum vs)) = false) as Hlt.
    { apply Z.ltb_ge. lia. }
    rewrite Hlt. reflexivity.
  - exfalso. apply Hsome. reflexivity.
Qed.

(* ================================================================= *)
(** ** §5 — Theorem: Halt when delta > epsilon (TH-3b)               *)
(* ================================================================= *)

Theorem th3b_halt_above_epsilon :
  forall s us vs,
    is_halted (ms_halt s) = false ->
    apply_updates (ms_validators s) us = Some vs ->
    delta_window (ms_window s) (v_sum vs) > epsilon ->
    ms_halt (step s us) = HR_LyapunovViolation.
Proof.
  intros s us vs Hnh Hau Hdelta.
  unfold step. rewrite Hnh. rewrite Hau.
  assert (Z.ltb epsilon (delta_window (ms_window s) (v_sum vs)) = true) as Hlt.
  { apply Z.ltb_lt. lia. }
  rewrite Hlt. reflexivity.
Qed.
