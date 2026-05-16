(** * QASH — Lyapunov Stability / Convergence Decrease (TH-3)

    File:    proofs/contractivity/lyapunov_stability.v
    Spec:    docs/spec/01_consensus.md §4a, §4b, §4c
    Class:   FORMAL THEOREM

    Theorems proved
    ---------------
    TH-3a  δ_window ≤ ε  ⟹  no halt triggered
    TH-3b  Convergence gate: the protocol halts iff
              V_current - min(window) > ε
    TH-3c  FinalizeEpoch (all-zero update) drives V_convergence to 0

    Constants (match GENESIS_CONSTANTS.toml and lyapunov.rs — v1.0 genesis)
    ---------------------------------------------------------
    weight_D  = 400 000    (α)
    weight_C  = 350 000    (β)
    weight_S  = 250 000    (γ)
    epsilon   =  20 000    (ε)
    scale     = 1 000 000  (𝔽_p denominator)
    window    = 3          (W)

    Note on weight_CH: the cascade-health term (χ) is specified in v1.1
    and activates when WEIGHT_BH > 0. It is not part of the v1.0 genesis
    Lyapunov function and is therefore not modelled here.

    Proof strategy
    --------------
    1.  Model V_convergence as a Z value (no floating point).
    2.  Define the three-slot window as a 3-tuple (head, mid, tail).
    3.  Prove min_window is well-defined and ≤ every element.
    4.  Prove TH-3a: if delta ≤ ε then halt_triggered = false.
    5.  Prove TH-3b: halt_triggered ↔ delta > ε (equivalence).
    6.  Prove TH-3c: finalize_epoch (D=0, C=0) yields V_convergence = 0.

    Status: All theorems fully proved. No Admitted markers.
    All three theorems are weight-independent: TH-3a/TH-3b depend only on
    the delta/epsilon relationship; TH-3c holds because finalization sets
    all metric components to 0 regardless of weight values.
*)

Require Import Coq.ZArith.ZArith.
Require Import Coq.Bool.Bool.
Require Import Coq.Lists.List.
Require Import Coq.micromega.Lia.
Open Scope Z_scope.

(* ================================================================= *)
(** ** §0 — Protocol Constants (v1.0 genesis)                         *)
(* ================================================================= *)

Definition weight_D : Z := 400_000.
Definition weight_C : Z := 350_000.
Definition weight_S : Z := 250_000.
Definition epsilon  : Z :=  20_000.
Definition scale    : Z := 1_000_000.

Lemma weight_D_pos : 0 < weight_D. Proof. unfold weight_D. lia. Qed.
Lemma weight_C_pos : 0 < weight_C. Proof. unfold weight_C. lia. Qed.
Lemma weight_S_pos : 0 < weight_S. Proof. unfold weight_S. lia. Qed.
Lemma epsilon_pos  : 0 < epsilon.  Proof. unfold epsilon.  lia. Qed.
Lemma scale_pos    : 0 < scale.    Proof. unfold scale.    lia. Qed.

Lemma zero_le_scale : 0 <= scale.
Proof. apply Z.lt_le_incl. apply scale_pos. Qed.

(* ================================================================= *)
(** ** §1 — Per-Validator Contribution                                *)
(* ================================================================= *)

(** Well-formed validator metrics: D and C live in [0, scale].
    Note: slash accumulator (S) and cascade health (CH) are tracked separately
    and do not appear in V_convergence at v1.0 genesis. *)
Record ValidatorMetrics := mkVM {
  vm_D : Z;
  vm_C : Z;
  vm_D_lo : 0 <= vm_D;
  vm_D_hi : vm_D <= scale;
  vm_C_lo : 0 <= vm_C;
  vm_C_hi : vm_C <= scale;
}.

(** V contribution of one validator epoch (v1.0: D and C terms only). *)
Definition v_validator (v : ValidatorMetrics) : Z :=
  weight_D * vm_D v + weight_C * vm_C v.

Lemma v_validator_nonneg : forall v, 0 <= v_validator v.
Proof.
  intros v.
  pose proof (vm_D_lo v); pose proof (vm_C_lo v).
  unfold v_validator, weight_D, weight_C in *.
  lia.
Qed.

Lemma v_validator_bounded : forall v,
    v_validator v <= (weight_D + weight_C) * scale.
Proof.
  intros v.
  pose proof (vm_D_lo v); pose proof (vm_D_hi v).
  pose proof (vm_C_lo v); pose proof (vm_C_hi v).
  unfold v_validator, weight_D, weight_C, scale in *.
  lia.
Qed.

(* ================================================================= *)
(** ** §2 — Three-Slot Convergence Window                             *)
(* ================================================================= *)

(** The window holds the three most-recent V_convergence values.
    Slot 0 is newest (head), slot 2 is oldest (tail).
    All values are non-negative (V_convergence sums of non-neg terms). *)

Record Window := mkW {
  w0 : Z;  (* newest *)
  w1 : Z;
  w2 : Z;  (* oldest *)
  w0_nn : 0 <= w0;
  w1_nn : 0 <= w1;
  w2_nn : 0 <= w2;
}.

Definition min3 (a b c : Z) : Z :=
  if a <=? b then
    if a <=? c then a else c
  else
    if b <=? c then b else c.

Lemma min3_le_left  : forall a b c, min3 a b c <= a.
Proof.
  intros a b c. unfold min3.
  destruct (Z.leb_spec a b); destruct (Z.leb_spec a c);
    destruct (Z.leb_spec b c); lia.
Qed.

Lemma min3_le_mid   : forall a b c, min3 a b c <= b.
Proof.
  intros a b c. unfold min3.
  destruct (Z.leb_spec a b); destruct (Z.leb_spec a c);
    destruct (Z.leb_spec b c); lia.
Qed.

Lemma min3_le_right : forall a b c, min3 a b c <= c.
Proof.
  intros a b c. unfold min3.
  destruct (Z.leb_spec a b); destruct (Z.leb_spec a c);
    destruct (Z.leb_spec b c); lia.
Qed.

Lemma min3_nonneg : forall a b c,
    0 <= a -> 0 <= b -> 0 <= c -> 0 <= min3 a b c.
Proof.
  intros a b c Ha Hb Hc. unfold min3.
  destruct (Z.leb_spec a b); destruct (Z.leb_spec a c);
    destruct (Z.leb_spec b c); lia.
Qed.

Definition window_min (w : Window) : Z :=
  min3 (w0 w) (w1 w) (w2 w).

Lemma window_min_le_w0 : forall w, window_min w <= w0 w.
Proof. intro w. apply min3_le_left. Qed.

Lemma window_min_le_w1 : forall w, window_min w <= w1 w.
Proof. intro w. apply min3_le_mid. Qed.

Lemma window_min_le_w2 : forall w, window_min w <= w2 w.
Proof. intro w. apply min3_le_right. Qed.

Lemma window_min_nonneg : forall w, 0 <= window_min w.
Proof.
  intro w. apply min3_nonneg; [apply w0_nn | apply w1_nn | apply w2_nn].
Qed.

(* ================================================================= *)
(** ** §3 — Delta and Halt Gate                                       *)
(* ================================================================= *)

(** δ_window = V_current − min(window).
    The protocol halts iff δ_window > ε. *)

Definition delta_window (v_current : Z) (w : Window) : Z :=
  v_current - window_min w.

Definition halt_triggered (v_current : Z) (w : Window) : bool :=
  window_min w + epsilon <? v_current.

(** ** TH-3a: δ ≤ ε  ⟹  no halt *)
Theorem TH3a_no_halt_within_epsilon :
  forall v_current w,
    delta_window v_current w <= epsilon ->
    halt_triggered v_current w = false.
Proof.
  intros v w Hdelta.
  unfold halt_triggered, delta_window in *.
  apply Z.ltb_nlt. lia.
Qed.

(** ** TH-3b: halt_triggered is equivalent to δ > ε *)
Theorem TH3b_halt_iff_delta_exceeds_epsilon :
  forall v_current w,
    halt_triggered v_current w = true
    <->
    delta_window v_current w > epsilon.
Proof.
  intros v w. unfold halt_triggered, delta_window.
  split.
  - intro H. apply Z.ltb_lt in H. lia.
  - intro H. apply Z.ltb_lt. lia.
Qed.

(** Corollary: halt_triggered = false  ↔  δ ≤ ε *)
Corollary TH3b_no_halt_iff_within_epsilon :
  forall v_current w,
    halt_triggered v_current w = false
    <->
    delta_window v_current w <= epsilon.
Proof.
  intros v w. unfold halt_triggered, delta_window.
  rewrite Z.ltb_nlt. lia.
Qed.

(* ================================================================= *)
(** ** §4 — Window Advancement                                        *)
(* ================================================================= *)

(** Pushing a new value shifts the window: new head, old head → mid,
    old mid → tail; old tail is discarded. *)
Definition push_window (v_new : Z) (Hnn : 0 <= v_new) (w : Window) : Window :=
  mkW v_new (w0 w) (w1 w) Hnn (w0_nn w) (w1_nn w).

Lemma push_window_min_le_new :
  forall v_new Hnn w,
    window_min (push_window v_new Hnn w) <= v_new.
Proof.
  intros v_new Hnn w. simpl. apply min3_le_left.
Qed.

(* ================================================================= *)
(** ** §5 — TH-3c: FinalizeEpoch Resets V_convergence to Zero        *)
(* ================================================================= *)

(** FinalizeEpoch sets all validator D and C to zero.
    The resulting V_convergence contribution per validator is 0. *)

Definition finalize_metrics : ValidatorMetrics :=
  mkVM 0 0
       (Z.le_refl 0) zero_le_scale
       (Z.le_refl 0) zero_le_scale.

Theorem TH3c_finalize_zero :
  v_validator finalize_metrics = 0.
Proof.
  unfold v_validator, finalize_metrics. simpl. lia.
Qed.

(** For any list of finalized validators, the sum V_convergence = 0. *)
Fixpoint sum_validators (vs : list ValidatorMetrics) : Z :=
  match vs with
  | nil      => 0
  | v :: rest => v_validator v + sum_validators rest
  end.

Theorem TH3c_finalize_list_zero :
  forall n : nat,
    sum_validators (repeat finalize_metrics n) = 0.
Proof.
  induction n as [| n' IH].
  - reflexivity.
  - simpl. rewrite IH. lia.
Qed.

(* ================================================================= *)
(** ** §6 — Proof Dependency Summary                                  *)
(**
  TH-3a: delta_window ≤ ε  ⟹  halt_triggered = false
         Depends on: Z arithmetic (AX-1, implicit in Coq's Z)
         Weight-independent: proof holds for any positive weight values.

  TH-3b: halt_triggered = true  ↔  delta_window > ε
         Depends on: Z arithmetic (AX-1)
         Weight-independent.

  TH-3c: finalize_epoch ⟹  V_convergence = 0
         Depends on: AX-1 (multiplication by zero).
         Proof: weight_D × 0 + weight_C × 0 = 0.

  All three theorems: FORMAL, no Admitted markers, AX-1/AX-2 only.
  Weight constants: v1.0 genesis (D=400k, C=350k, S=250k).
*)
(* ================================================================= *)
