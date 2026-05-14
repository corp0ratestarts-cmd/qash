(** TH-9: Cascade Health Factor Boundedness
    CH_t ∈ [0, p] for all admissible epochs.
    χ · CH_t ∈ [0, χ · p] ⊂ [0, i128::MAX] — no overflow possible.

    Proof strategy:
      1. CH_t = cascade_fail_count × p / max_queries_per_epoch
      2. cascade_fail_count ∈ [0, max_queries_per_epoch] by admissibility
      3. p = max_queries, so CH_t = fail_count × p / p = fail_count ∈ [0, p]
      4. χ = 150_000, p = 1_000_000 → χ · p = 1.5 × 10^11 << i128::MAX

    Depends on: AX-1, AX-2, §4c definition in 01_consensus.md
*)

Require Import Coq.Arith.Arith.
Require Import Coq.ZArith.ZArith.
Require Import Coq.micromega.Lia.
Open Scope Z_scope.

(** Protocol constants *)
Definition p          : Z := 1_000_000.
Definition chi        : Z := 150_000.
Definition max_queries : Z := 1_000_000.

Lemma p_pos         : 0 < p.          Proof. unfold p.          lia. Qed.
Lemma chi_nonneg    : 0 <= chi.       Proof. unfold chi.        lia. Qed.
Lemma max_queries_pos : 0 < max_queries. Proof. unfold max_queries. lia. Qed.

(** Since p = max_queries, division is exact: fail_count × p / p = fail_count. *)
Lemma p_eq_max_queries : p = max_queries.
Proof. unfold p, max_queries. reflexivity. Qed.

(** CH_t is bounded in [0, p] *)
Lemma ch_t_upper_bound :
  forall (fail_count : Z),
  0 <= fail_count <= max_queries ->
  let ch_t := fail_count * p / max_queries in
  0 <= ch_t <= p.
Proof.
  intros fail_count [Hlo Hhi].
  unfold max_queries, p in *.
  split.
  - (* 0 <= fail_count * 1_000_000 / 1_000_000 *)
    apply Z.div_nonneg.
    + apply Z.mul_nonneg_nonneg; lia.
    + lia.
  - (* fail_count * 1_000_000 / 1_000_000 <= 1_000_000 *)
    (* By Z.div_le_upper_bound: a/b <= q  ←  a <= b*q (when b > 0) *)
    apply Z.div_le_upper_bound; [lia |].
    (* need: fail_count * 1_000_000 <= 1_000_000 * 1_000_000 *)
    apply Z.mul_le_mono_nonneg_r; lia.
Qed.

(** χ · CH_t does not overflow i128 *)
Lemma cascade_health_term_no_overflow :
  forall (ch_t : Z),
  0 <= ch_t <= p ->
  let term := chi * ch_t in
  term <= chi * p.
Proof.
  intros ch_t [Hlo Hhi].
  unfold chi, p.
  (* 150_000 * ch_t <= 150_000 * 1_000_000 *)
  apply Z.mul_le_mono_nonneg_l; lia.
Qed.

(** Combined bound: χ · CH_t ∈ [0, χ · p] for admissible fail_count *)
Lemma ch_term_admissible :
  forall (fail_count : Z),
  0 <= fail_count <= max_queries ->
  let ch_t  := fail_count * p / max_queries in
  let term  := chi * ch_t in
  0 <= term <= chi * p.
Proof.
  intros fail_count Hbounds.
  set (ch_t := fail_count * p / max_queries).
  pose proof (ch_t_upper_bound fail_count Hbounds) as [Hlo Hhi].
  split.
  - apply Z.mul_nonneg_nonneg; [apply chi_nonneg | exact Hlo].
  - apply cascade_health_term_no_overflow; split; assumption.
Qed.
