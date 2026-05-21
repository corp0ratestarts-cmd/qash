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
Import ListNotations.
Require Import Coq.micromega.Lia.
Open Scope Z_scope.

(* ================================================================= *)
(** ** §0 — Protocol Constants (v1.0 genesis, matching GENESIS_CONSTANTS.toml) *)
(* ================================================================= *)

Definition weight_D : Z := 400_000.
Definition weight_C : Z := 350_000.
Definition weight_S : Z := 250_000.
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
Definition fixed_mul (a b : Z) : Z :=
  (a * b) / scale.

(** V(v) = floor(α·D/scale) + floor(β·C/scale) per validator.
    Matches FixedPoint::checked_mul and lyapunov.rs evaluate(). *)
Definition v_validator (v : ValidatorMetrics) : Z :=
  fixed_mul weight_D (vm_D v) + fixed_mul weight_C (vm_C v).

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
(** ** §1a — Rust-identifier aliases and encoding subset              *)
(* ================================================================= *)

(** The following definitions intentionally use names that mirror Rust
    identifiers in crates/consensus/src/{lyapunov,transition,encoding}.rs.
    They are a small, executable refinement surface: encoding first, then the
    Lyapunov transition observation subset. *)

Definition SCALE : Z := scale.
Definition WEIGHT_D : Z := weight_D.
Definition WEIGHT_C : Z := weight_C.
Definition WEIGHT_S : Z := weight_S.
Definition EPSILON : Z := epsilon.
Definition WINDOW_SIZE_WIRE : Z := Z.of_nat window_sz.
Definition WINDOW_SIZE : nat := window_sz.

Definition ValidatorMetrics_ZERO : ValidatorMetrics := zero_vm.
Definition ConvergenceWindow := ConvWindow.
Definition ConvergenceWindow_new : ConvergenceWindow := empty_window.
Definition ConvergenceWindow_push := cw_push.
Definition ConvergenceWindow_is_full := cw_is_full.
Definition ConvergenceWindow_min_value := cw_min.
Definition compute_delta_window (v_current : Z) (window : ConvergenceWindow) : Z :=
  delta_window window v_current.

Record LyapunovEval := mkLyapunovEval {
  lyapunov_v_convergence : Z;
  lyapunov_phi_safety : Z;
  lyapunov_v_total : Z;
  lyapunov_delta_window : Z;
  lyapunov_halt_triggered : bool;
}.

Fixpoint max_slash (vs : list ValidatorMetrics) : Z :=
  match vs with
  | [] => 0
  | v :: rest => Z.max (vm_S v) (max_slash rest)
  end.

Definition evaluate (validators : list ValidatorMetrics) (window : ConvergenceWindow)
    : LyapunovEval :=
  let v_conv := v_sum validators in
  let phi := fixed_mul WEIGHT_S (max_slash validators) in
  let total := v_conv + phi in
  let delta := compute_delta_window v_conv window in
  mkLyapunovEval v_conv phi total delta (Z.ltb EPSILON delta).

Definition EpochState := ModelState.
Definition EpochInput := list ValidatorUpdate.
Definition advance_epoch := step.

Definition ENCODING_VERSION : Z := 0.
Definition STATE_HEADER_SIZE : Z := 52.
Definition VALIDATOR_DYNAMIC_SIZE : Z := 48.

Fixpoint le_bytes (n : Z) (len : nat) : list Z :=
  match len with
  | O => []
  | S len' => Z.modulo n 256 :: le_bytes (Z.div n 256) len'
  end.

Definition encode_u32 (n : Z) : list Z := le_bytes n 4.
Definition encode_u64 (n : Z) : list Z := le_bytes n 8.
Definition encode_i128 (n : Z) : list Z := le_bytes n 16.

Definition encode_state_header
    (epoch : Z) (validator_count : Z) (halt_reason : Z) (entropy_seed : list Z)
    : list Z :=
  encode_u32 ENCODING_VERSION ++
  encode_u64 epoch ++
  encode_u32 validator_count ++
  [halt_reason; 0; 0; 0] ++
  entropy_seed.

Definition compute_leaf_index (validator_id epoch : Z) (epoch_seed : list Z) : list Z :=
  encode_u64 validator_id ++ encode_u64 epoch ++ epoch_seed.

Definition encode_validator_dynamic (v : ValidatorMetrics) : list Z :=
  encode_i128 (vm_D v) ++ encode_i128 (vm_C v) ++ encode_i128 (vm_S v).

Record AdvanceEpochObservation := mkAdvanceEpochObservation {
  obs_epoch : Z;
  obs_halt_reason : HaltReason;
  obs_v_convergence : Z;
  obs_delta_window : Z;
  obs_window_values : list Z;
}.

Definition advance_epoch_observation (s : EpochState) (updates : list ValidatorUpdate)
    : AdvanceEpochObservation :=
  let projected :=
    match apply_updates (ms_validators s) updates with
    | Some vs => vs
    | None => ms_validators s
    end in
  let lyap := evaluate projected (ms_window s) in
  let s' := advance_epoch s updates in
  mkAdvanceEpochObservation
    (ms_epoch s')
    (ms_halt s')
    (lyapunov_v_convergence lyap)
    (lyapunov_delta_window lyap)
    (cw_values (ms_window s')).

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


(* ================================================================= *)
(** ** §6 — Checked correspondence vectors for the initial subset     *)
(* ================================================================= *)

Definition zero_seed_32 : list Z := repeat 0 32.
Definition rust_header_tv0 : list Z :=
  encode_state_header 1 4 0 zero_seed_32.

Example encode_state_header_tv0_checked :
  rust_header_tv0 =
    [0;0;0;0; 1;0;0;0;0;0;0;0; 4;0;0;0; 0;0;0;0] ++ repeat 0 32.
Proof. reflexivity. Qed.

Example compute_leaf_index_tv0_checked :
  compute_leaf_index 7 1 zero_seed_32 =
    [7;0;0;0;0;0;0;0; 1;0;0;0;0;0;0;0] ++ repeat 0 32.
Proof. reflexivity. Qed.

Example encode_validator_dynamic_tv0_checked :
  encode_validator_dynamic (mkVM 500_000 250_000 1_000) =
    [32;161;7;0;0;0;0;0;0;0;0;0;0;0;0;0] ++
    [144;208;3;0;0;0;0;0;0;0;0;0;0;0;0;0] ++
    [232;3;0;0;0;0;0;0;0;0;0;0;0;0;0;0].
Proof. reflexivity. Qed.

Definition genesis4 : EpochState :=
  mkMS 0 HR_None [zero_vm; zero_vm; zero_vm; zero_vm] empty_window.
Definition genesis1 : EpochState :=
  mkMS 0 HR_None [zero_vm] empty_window.

Definition idle4 : list ValidatorUpdate := [VU_Idle; VU_Idle; VU_Idle; VU_Idle].

Example advance_epoch_idle4_observation_checked :
  advance_epoch_observation genesis4 idle4 =
    mkAdvanceEpochObservation 1 HR_None 0 0 [0].
Proof. reflexivity. Qed.

Definition spike4_900k : list ValidatorUpdate :=
  [VU_Update 900_000 900_000 0;
   VU_Update 900_000 900_000 0;
   VU_Update 900_000 900_000 0;
   VU_Update 900_000 900_000 0].

Definition filled_zero_window_state : EpochState := run genesis4 [idle4; idle4; idle4].

Example advance_epoch_lyapunov_halt_observation_checked :
  advance_epoch_observation filled_zero_window_state spike4_900k =
    mkAdvanceEpochObservation 3 HR_LyapunovViolation 2_700_000 2_700_000 [0;0;0].
Proof. reflexivity. Qed.

Definition vm_v100k : ValidatorMetrics := mkVM 250_000 0 0.
Definition epsilon_boundary_state : EpochState :=
  mkMS 0 HR_None [vm_v100k] (mkCW [100_000; 100_000; 100_000]).
Definition epsilon_boundary_update : list ValidatorUpdate :=
  [VU_Update 300_000 0 0].

Example advance_epoch_epsilon_boundary_observation_checked :
  advance_epoch_observation epsilon_boundary_state epsilon_boundary_update =
    mkAdvanceEpochObservation 1 HR_None 120_000 20_000 [120_000;100_000;100_000].
Proof. reflexivity. Qed.

Definition invalid_negative_update : list ValidatorUpdate :=
  [VU_Update (-1) 0 0].

Example advance_epoch_decode_invalid_observation_checked :
  advance_epoch_observation genesis1 invalid_negative_update =
    mkAdvanceEpochObservation 0 HR_DecodeInvalid 0 0 [].
Proof. reflexivity. Qed.

Definition halted_absorbing_state : EpochState :=
  mkMS 7 HR_LyapunovViolation [zero_vm] (mkCW [1;2;3]).

Example advance_epoch_halted_absorbing_observation_checked :
  advance_epoch_observation halted_absorbing_state [VU_Update 300_000 0 0] =
    mkAdvanceEpochObservation 7 HR_LyapunovViolation 120_000 119_999 [1;2;3].
Proof. reflexivity. Qed.
