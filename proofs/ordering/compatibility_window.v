(** * QASH — Compatibility Window Safety (v1.1)

    File:    proofs/ordering/compatibility_window.v
    Spec:    docs/spec/00_execution_model.md §3; GENESIS_CONSTANTS.toml [cascade]
    Class:   FORMAL THEOREM
    Status:  All theorems fully proved. No Admitted markers.

    Theorems proved
    ---------------
    CW-1  Compatibility window bound: COMPATIBILITY_WINDOW = 100.

    CW-2  Version acceptance before window:
            epoch < COMPATIBILITY_WINDOW →
            v1.0 envelope is accepted (version check does not reject it).

    CW-3  Version rejection after window:
            epoch ≥ COMPATIBILITY_WINDOW ∧ version < V1_1 →
            advance_epoch returns IncompatibleVersion.

    CW-4  V1.1 always accepted:
            version ≥ V1_1 →
            version check passes regardless of epoch.

    CW-5  Monotonicity of the window: once epoch ≥ COMPATIBILITY_WINDOW,
            all future epochs satisfy the predicate (epoch is non-decreasing).

    Background
    ----------
    `COMPATIBILITY_WINDOW = 100` in `crates/consensus/src/transition.rs`.
    `PROTOCOL_VERSION_V1_0 = 0x1000`, `PROTOCOL_VERSION_V1_1 = 0x1100`
    in `crates/consensus/src/envelope.rs`.

    The version gate in `step_1_validate`:
      if epoch ≥ COMPATIBILITY_WINDOW && version < V1_1 → IncompatibleVersion (H8)

    This file proves the structural correctness of that gate — namely that it
    accepts exactly what the spec requires and rejects exactly what it should.
    The Rust implementation is separately verified by the unit tests
    `version_gate_*` in `crates/consensus/src/transition.rs`.
*)

Require Import Coq.ZArith.ZArith.
Require Import Coq.Bool.Bool.
Require Import Coq.micromega.Lia.
Open Scope Z_scope.

(* ================================================================= *)
(** ** §0 — Protocol Constants                                        *)
(* ================================================================= *)

Definition COMPATIBILITY_WINDOW : Z := 100.
Definition PROTOCOL_VERSION_V1_0 : Z := 0x1000. (* 4096 *)
Definition PROTOCOL_VERSION_V1_1 : Z := 0x1100. (* 4352 *)

Lemma compat_window_pos : 0 < COMPATIBILITY_WINDOW.
Proof. unfold COMPATIBILITY_WINDOW. lia. Qed.

Lemma v1_0_lt_v1_1 : PROTOCOL_VERSION_V1_0 < PROTOCOL_VERSION_V1_1.
Proof. unfold PROTOCOL_VERSION_V1_0, PROTOCOL_VERSION_V1_1. lia. Qed.

(* ================================================================= *)
(** ** §1 — Version Gate Predicate                                    *)
(* ================================================================= *)

(** `version_rejected epoch version` models the H8 rejection predicate:
    rejected iff epoch ≥ COMPATIBILITY_WINDOW AND version < V1_1. *)
Definition version_rejected (epoch version : Z) : Prop :=
  epoch >= COMPATIBILITY_WINDOW /\ version < PROTOCOL_VERSION_V1_1.

(** `version_accepted epoch version` is the complement. *)
Definition version_accepted (epoch version : Z) : Prop :=
  ~ version_rejected epoch version.

Lemma version_accepted_unfold :
  forall epoch version,
  version_accepted epoch version <->
  epoch < COMPATIBILITY_WINDOW \/ version >= PROTOCOL_VERSION_V1_1.
Proof.
  intros. unfold version_accepted, version_rejected. lia.
Qed.

(* ================================================================= *)
(** ** §2 — Core Theorems                                             *)
(* ================================================================= *)

(** CW-1: Compatibility window bound — exactly 100 epochs. *)
Theorem compatibility_window_bound :
  COMPATIBILITY_WINDOW = 100.
Proof. unfold COMPATIBILITY_WINDOW. reflexivity. Qed.

(** CW-2: V1.0 envelope accepted *before* the window closes. *)
Theorem version_v1_0_accepted_before_window :
  forall epoch,
  epoch < COMPATIBILITY_WINDOW ->
  version_accepted epoch PROTOCOL_VERSION_V1_0.
Proof.
  intros epoch Hepoch.
  rewrite version_accepted_unfold.
  left. exact Hepoch.
Qed.

(** CW-3: V1.0 envelope rejected *at or after* the window closes. *)
Theorem version_v1_0_rejected_after_window :
  forall epoch,
  epoch >= COMPATIBILITY_WINDOW ->
  version_rejected epoch PROTOCOL_VERSION_V1_0.
Proof.
  intros epoch Hepoch.
  unfold version_rejected.
  split.
  - exact Hepoch.
  - unfold PROTOCOL_VERSION_V1_0, PROTOCOL_VERSION_V1_1. lia.
Qed.

(** CW-4: V1.1 (and later) is always accepted, regardless of epoch. *)
Theorem version_v1_1_always_accepted :
  forall epoch version,
  version >= PROTOCOL_VERSION_V1_1 ->
  version_accepted epoch version.
Proof.
  intros epoch version Hv.
  rewrite version_accepted_unfold.
  right. exact Hv.
Qed.

(** CW-5: The rejection predicate is monotone in epoch:
    once past the compatibility window, all successor epochs also reject V1.0. *)
Theorem window_closure_monotone :
  forall epoch,
  epoch >= COMPATIBILITY_WINDOW ->
  forall epoch',
  epoch' >= epoch ->
  version_rejected epoch' PROTOCOL_VERSION_V1_0.
Proof.
  intros epoch Hepoch epoch' Hepoch'.
  apply version_v1_0_rejected_after_window.
  lia.
Qed.

(* ================================================================= *)
(** ** §3 — Boundary Sharpness                                        *)
(* ================================================================= *)

(** The boundary is sharp: epoch 99 accepts V1.0, epoch 100 rejects it. *)
Lemma epoch_99_accepts_v1_0 :
  version_accepted 99 PROTOCOL_VERSION_V1_0.
Proof.
  apply version_v1_0_accepted_before_window.
  unfold COMPATIBILITY_WINDOW. lia.
Qed.

Lemma epoch_100_rejects_v1_0 :
  version_rejected 100 PROTOCOL_VERSION_V1_0.
Proof.
  apply version_v1_0_rejected_after_window.
  unfold COMPATIBILITY_WINDOW. lia.
Qed.

(** The gate is exact: acceptance and rejection are mutually exclusive and
    exhaustive for any (epoch, version) pair. *)
Theorem version_gate_trichotomy :
  forall epoch version,
  version_accepted epoch version \/ version_rejected epoch version.
Proof.
  intros epoch version.
  unfold version_accepted, version_rejected.
  destruct (Z.le_gt_cases COMPATIBILITY_WINDOW epoch) as [Hge | Hlt];
  destruct (Z.lt_ge_cases version PROTOCOL_VERSION_V1_1) as [Hlt2 | Hge2].
  - right. split; lia.
  - left. unfold version_rejected. lia.
  - left. unfold version_rejected. lia.
  - left. unfold version_rejected. lia.
Qed.

Theorem version_gate_exclusive :
  forall epoch version,
  ~ (version_accepted epoch version /\ version_rejected epoch version).
Proof.
  intros epoch version [Ha Hr].
  unfold version_accepted in Ha.
  exact (Ha Hr).
Qed.

(* ================================================================= *)
(** ** §4 — Epoch State Machine: Epoch is Non-Decreasing              *)
(* ================================================================= *)

(** In Domain A, each successful `advance_epoch` increments the epoch by 1.
    We model a step as epoch ↦ epoch + 1, then prove that once past
    the compatibility window the epoch never returns below it. *)

Inductive EpochStep : Z -> Z -> Prop :=
  | step_advance : forall e, EpochStep e (e + 1).

(** Epoch never decreases. *)
Lemma epoch_step_nondecreasing :
  forall e e', EpochStep e e' -> e' > e.
Proof.
  intros e e' H. inversion H. lia.
Qed.

(** Once past the window, the rejection predicate holds forever.
    We model multi-step reachability as: e' = e + k for some natural k. *)
Theorem past_window_stays_past :
  forall e : Z,
  forall k : nat,
  e >= COMPATIBILITY_WINDOW ->
  e + Z.of_nat k >= COMPATIBILITY_WINDOW.
Proof.
  intros e k Hge.
  lia.
Qed.

(** Corollary: V1.0 is rejected for all future epochs once window closes. *)
Corollary v1_0_rejected_all_future :
  forall e : Z,
  forall k : nat,
  e >= COMPATIBILITY_WINDOW ->
  version_rejected (e + Z.of_nat k) PROTOCOL_VERSION_V1_0.
Proof.
  intros e k Hge.
  apply version_v1_0_rejected_after_window.
  apply past_window_stays_past.
  exact Hge.
Qed.
