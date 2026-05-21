(** * QASH — TH-3 System Closure

    This file connects the executable transition model to the TH-3 convergence
    envelope. The local TH-3 lemmas prove the arithmetic gate; this artifact
    proves that the composed model step uses that gate at the transition
    boundary.

    Status: FORMAL. No Admitted markers.
*)

Require Import Coq.ZArith.ZArith.
Require Import Coq.Bool.Bool.
Require Import Coq.Lists.List.
Require Import Coq.micromega.Lia.
Require Import QASH.model.Model.
Open Scope Z_scope.

(** If a composed executable step succeeds, the pre-step convergence window and
    post-update validator projection are inside the epsilon envelope. *)
Theorem composed_step_success_within_epsilon :
  forall (s s' : ModelState) (updates : list ValidatorUpdate),
    step s updates = s' ->
    ms_halt s' = HR_None ->
    delta_window (ms_window s) (v_sum (ms_validators s')) <= epsilon.
Proof.
  intros s s' updates Hstep Hhalt.
  unfold step in Hstep.
  destruct (is_halted (ms_halt s)) eqn:Hhalted.
  - subst s'. rewrite Hhalt in Hhalted. discriminate Hhalted.
  - destruct (apply_updates (ms_validators s) updates) as [new_vs|] eqn:Hupdates.
    + set (v_cur := v_sum new_vs) in *.
      set (delta := delta_window (ms_window s) v_cur) in *.
      destruct (epsilon <? delta) eqn:Hgate.
      * subst s'. simpl in Hhalt. discriminate Hhalt.
      * subst s'. simpl.
        apply Z.ltb_ge in Hgate.
        exact Hgate.
    + subst s'. simpl in Hhalt. discriminate Hhalt.
Qed.

(** If a running composed step halts with a Lyapunov violation, the same
    projection exceeded epsilon before state advancement. *)
Theorem composed_step_lyapunov_halt_exceeds_epsilon :
  forall (s s' : ModelState) (updates : list ValidatorUpdate),
    is_halted (ms_halt s) = false ->
    step s updates = s' ->
    ms_halt s' = HR_LyapunovViolation ->
    exists new_vs,
      apply_updates (ms_validators s) updates = Some new_vs /\
      epsilon < delta_window (ms_window s) (v_sum new_vs).
Proof.
  intros s s' updates Hrunning Hstep Hhalt.
  unfold step in Hstep.
  rewrite Hrunning in Hstep.
  destruct (apply_updates (ms_validators s) updates) as [new_vs|] eqn:Hupdates.
  - set (v_cur := v_sum new_vs) in *.
    set (delta := delta_window (ms_window s) v_cur) in *.
    destruct (epsilon <? delta) eqn:Hgate.
    + exists new_vs. split; [reflexivity |].
      apply Z.ltb_lt in Hgate. exact Hgate.
    + subst s'. simpl in Hhalt. discriminate Hhalt.
  - subst s'. simpl in Hhalt. discriminate Hhalt.
Qed.

(** Abstract §A8 composition rule: when every admitted transaction effect is
    non-increasing on V_convergence, their composition is also non-increasing.
    TX-0 supplies equality and TX-1 supplies non-increase, so both instantiate
    this rule with zero positive perturbation. *)
Theorem nonincreasing_effects_compose :
  forall before after_tx after_all : Z,
    after_tx <= before ->
    after_all <= after_tx ->
    after_all <= before.
Proof.
  intros before after_tx after_all Htx Hall.
  apply Z.le_trans with (m := after_tx); assumption.
Qed.

(** Global epsilon envelope rule used by the runtime/test correspondence: a
    composed transition whose projection stays within epsilon cannot be reported
    as a Lyapunov halt by the executable model gate. *)
Theorem epsilon_envelope_excludes_lyapunov_halt :
  forall (s s' : ModelState) (updates : list ValidatorUpdate),
    is_halted (ms_halt s) = false ->
    step s updates = s' ->
    (forall new_vs,
        apply_updates (ms_validators s) updates = Some new_vs ->
        delta_window (ms_window s) (v_sum new_vs) <= epsilon) ->
    ms_halt s' <> HR_LyapunovViolation.
Proof.
  intros s s' updates Hrunning Hstep Henvelope Hbad.
  destruct (composed_step_lyapunov_halt_exceeds_epsilon
              s s' updates Hrunning Hstep Hbad) as
      [new_vs [Hupdates Hdelta]].
  pose proof (Henvelope new_vs Hupdates) as Hwithin.
  lia.
Qed.
