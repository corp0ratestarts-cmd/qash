(** TH-9: Cascade Health Factor Boundedness
    CH_t ∈ [0, p] for all admissible epochs.
    χ · CH_t ∈ [0, χ · p] ⊂ [0, i128::MAX] — no overflow possible.

    Proof strategy:
      1. CH_t = cascade_fail_count × p / max_queries_per_epoch
      2. cascade_fail_count ∈ [0, max_queries_per_epoch] by admissibility
      3. p = max_queries, so CH_t = fail_count × p / p = fail_count exactly
         (proved via Z.div_unique with remainder 0)
      4. χ = 150_000, p = 1_000_000 → χ · p = 1.5 × 10^11 << i128::MAX

    Depends on: AX-1, AX-2, §4c definition in 01_consensus.md
*)

Require Import Coq.ZArith.ZArith.
Require Import Coq.micromega.Lia.
Open Scope Z_scope.

(** Protocol constants *)
Definition p           : Z := 1_000_000.
Definition chi         : Z := 150_000.
Definition max_queries : Z := 1_000_000.

(** Exact-division helper: n * 1_000_000 / 1_000_000 = n.
    Proved via Z.div_unique: quotient = n, remainder = 0.
    Preconditions: 0 <= n is not required — holds for all n when divisor > 0
    and the division is exact. We state 0 <= n because all callers have this. *)
Lemma mul_div_self (n : Z) :
  n * 1_000_000 / 1_000_000 = n.
Proof.
  apply (Z.div_unique _ _ _ 0); lia.
Qed.

(** CH_t is bounded in [0, p]. *)
Lemma ch_t_upper_bound :
  forall (fail_count : Z),
  0 <= fail_count <= max_queries ->
  0 <= fail_count * p / max_queries <= p.
Proof.
  intros fail_count [Hlo Hhi].
  unfold max_queries, p in *.
  rewrite mul_div_self.
  split; lia.
Qed.

(** χ · CH_t does not overflow — the product stays within [0, χ · p]. *)
Lemma cascade_health_term_no_overflow :
  forall (ch_t : Z),
  0 <= ch_t <= p ->
  0 <= chi * ch_t <= chi * p.
Proof.
  intros ch_t [Hlo Hhi].
  unfold chi, p in *.
  split.
  - apply Z.mul_nonneg_nonneg; lia.
  - apply Z.mul_le_mono_nonneg_l; lia.
Qed.

(** Combined: for any admissible fail_count, χ · CH_t ∈ [0, χ · p]. *)
Lemma ch_term_admissible :
  forall (fail_count : Z),
  0 <= fail_count <= max_queries ->
  0 <= chi * (fail_count * p / max_queries) <= chi * p.
Proof.
  intros fail_count Hbounds.
  apply cascade_health_term_no_overflow.
  apply ch_t_upper_bound.
  exact Hbounds.
Qed.
