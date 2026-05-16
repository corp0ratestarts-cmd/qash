(** * QASH — Lyapunov Grace Convergence (TH-GC)

    File:    proofs/contractivity/lyapunov_grace_convergence.v
    Spec:    docs/spec/11_security_model.md §11.5
             docs/spec/01_consensus.md §5 (ε_honest / ε_halt margin)
             docs/spec/02_transition_axioms.md §A8
    Class:   FORMAL THEOREM (no Admitted markers)

    Theorem TH-GC (Grace Convergence — Tolerance Pillar)
    -----------------------------------------------------
    If each epoch's V_convergence increases by at most ε_honest (= 2 000),
    then over a full 3-epoch window the δ_window excursion is bounded by
    3 × ε_honest = 6 000, which is strictly less than ε_halt = 20 000.
    Therefore halt_triggered = false.

    This is the formal proof of the TOLERANCE pillar of the Bat Immunology
    Divergence Containment model (spec §11.5).  It shows that bounded honest
    behaviour (staying within the ε_honest budget per epoch) can never trigger
    the convergence halt gate (H1), because the 10× safety margin between
    ε_honest and ε_halt absorbs the cumulative window excursion.

    Corollary
    ---------
    tolerance_margin_remaining = max(0, ε_halt − δ_window) ≥ ε_halt − 3×ε_honest
    = 14 000 > 0 under honest operation. The margin is never exhausted.

    Dependencies
    ------------
    Imports lyapunov_stability.v for TH-3a, min3 lemmas, and Window/delta_window
    definitions.  No new axioms introduced.

    Status: Fully proved. No Admitted markers.
*)

Require Import Coq.ZArith.ZArith.
Require Import Coq.Bool.Bool.
Require Import Coq.micromega.Lia.
Require Import QASH.contractivity.lyapunov_stability.
Open Scope Z_scope.

(* ================================================================= *)
(** ** §0 — Honest-validator constant (spec §A8)                      *)
(* ================================================================= *)

(** ε_honest = 2 000: the per-epoch perturbation budget for honest validators.
    Satisfies: WINDOW_SIZE × ε_honest = 3 × 2 000 = 6 000 ≤ ε_halt = 20 000. *)
Definition epsilon_honest : Z := 2_000.

Lemma epsilon_honest_pos : 0 < epsilon_honest.
Proof. unfold epsilon_honest. lia. Qed.

(** Key arithmetic fact: 3 honest epochs cannot accumulate enough excursion
    to reach the halt threshold. *)
Lemma window_honest_budget_within_epsilon :
  3 * epsilon_honest <= epsilon.
Proof. unfold epsilon_honest, epsilon. lia. Qed.

(** Strict ordering invariant (also enforced by compile-time assert in params.rs). *)
Lemma epsilon_honest_lt_epsilon :
  epsilon_honest < epsilon.
Proof. unfold epsilon_honest, epsilon. lia. Qed.

(** Tolerance margin lower bound under honest operation. *)
Lemma tolerance_margin_lower_bound :
  epsilon - 3 * epsilon_honest > 0.
Proof. unfold epsilon, epsilon_honest. lia. Qed.

(* ================================================================= *)
(** ** §1 — Core window excursion bound                               *)
(* ================================================================= *)

(** If the window was built by steps bounded by ε_honest (oldest w2 → middle w1
    → newest w0 → v_new), the δ_window excursion ≤ 3 × ε_honest.

    window_min uses min3(w0, w1, w2) (newest-first ordering, matching push_window). *)

Lemma TH_GC_window_bounded_steps :
  forall (w : Window) (v_new : Z),
    0 <= v_new ->
    (* w2 = oldest, w1 = middle, w0 = newest preceding epoch *)
    w1 w <= w2 w + epsilon_honest ->
    w0 w <= w1 w + epsilon_honest ->
    v_new <= w0 w + epsilon_honest ->
    v_new - window_min w <= 3 * epsilon_honest.
Proof.
  intros w v_new Hnn H12 H01 H0n.
  unfold window_min, min3, epsilon_honest in *.
  pose proof (w0_nn w) as Hw0.
  pose proof (w1_nn w) as Hw1.
  pose proof (w2_nn w) as Hw2.
  destruct (Z.leb_spec (w0 w) (w1 w));
  destruct (Z.leb_spec (w0 w) (w2 w));
  destruct (Z.leb_spec (w1 w) (w2 w)); lia.
Qed.

(* ================================================================= *)
(** ** §2 — TH-GC: No halt under honest operation                     *)
(* ================================================================= *)

(** TH-GC (direct form): excursion bounded by 3×ε_honest ⟹ no halt. *)
Theorem TH_GC_grace_no_halt :
  forall (v_new : Z) (w : Window),
    0 <= v_new ->
    v_new - window_min w <= 3 * epsilon_honest ->
    halt_triggered v_new w = false.
Proof.
  intros v_new w Hnn Hbound.
  apply TH3a_no_halt_within_epsilon.
  unfold delta_window.
  apply Z.le_trans with (m := 3 * epsilon_honest).
  - exact Hbound.
  - exact window_honest_budget_within_epsilon.
Qed.

(** TH-GC (step form): main theorem.  Given a full 3-epoch window built by
    bounded honest steps, the H1 halt gate is not triggered. *)
Theorem TH_GC_honest_steps_no_halt :
  forall (w : Window) (v_new : Z),
    0 <= v_new ->
    w1 w <= w2 w + epsilon_honest ->
    w0 w <= w1 w + epsilon_honest ->
    v_new <= w0 w + epsilon_honest ->
    halt_triggered v_new w = false.
Proof.
  intros w v_new Hnn H12 H01 H0n.
  apply TH_GC_grace_no_halt.
  - exact Hnn.
  - exact (TH_GC_window_bounded_steps w v_new Hnn H12 H01 H0n).
Qed.

(* ================================================================= *)
(** ** §3 — Tolerance margin corollary                                *)
(* ================================================================= *)

(** Under honest operation, the tolerance margin remaining is at least
    ε_halt − 3×ε_honest = 20 000 − 6 000 = 14 000 > 0. *)
Theorem TH_GC_tolerance_margin_positive :
  forall (w : Window) (v_new : Z),
    0 <= v_new ->
    w1 w <= w2 w + epsilon_honest ->
    w0 w <= w1 w + epsilon_honest ->
    v_new <= w0 w + epsilon_honest ->
    epsilon - delta_window v_new w >= epsilon - 3 * epsilon_honest.
Proof.
  intros w v_new Hnn H12 H01 H0n.
  unfold delta_window.
  pose proof (TH_GC_window_bounded_steps w v_new Hnn H12 H01 H0n) as Hexc.
  lia.
Qed.

(* ================================================================= *)
(** ** §4 — Proof dependency summary                                  *)
(**
  TH-GC (honest_steps_no_halt):
    If ∀ epoch t in window, ΔV(t) ≤ ε_honest
    Then δ_window ≤ 3 × ε_honest = 6 000 < 20 000 = ε_halt
    Therefore halt_triggered = false (from TH-3a)

  TH-GC (tolerance_margin_positive):
    Under the same conditions, tolerance_margin ≥ 14 000 > 0.

  Dependencies:
    AX-1 (Z arithmetic, via Coq's Z module)
    TH-3a (from lyapunov_stability.v — fully proved)
    ε_honest = 2 000 (spec §A8 — genesis constant)

  No new axioms. No Admitted markers.
  The 10× safety margin (ε_halt / ε_honest = 10) provides structural
  tolerance against bounded ISA variance, transient cryptographic noise,
  and multi-epoch perturbation accumulation.
*)
(* ================================================================= *)
