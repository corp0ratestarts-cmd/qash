(** crypto_game_framework.v — Shared cryptographic advantage type.

    Purpose:
      Provide a common vocabulary for quantitative security bounds used across
      the blinding and IT-MAC proof files.  Expressing advantage as a concrete
      ratio (rather than the vacuous `True`) lets Coq type-check that axiom
      statements are well-formed and that derived theorems follow the right
      arithmetic structure.

    Scope:
      This file only defines types and arithmetic helpers.  All cryptographic
      hardness assumptions are stated in the files that require them.
*)

Require Import Coq.ZArith.ZArith.
Require Import Lia.
Open Scope Z_scope.

(* ---------------------------------------------------------------------------
   Advantage as a rational number p / q with q > 0.
   We avoid Coq's QArith to stay in Z for all arithmetic.
   --------------------------------------------------------------------------- *)

Record Advantage : Type := mkAdvantage {
  adv_num : Z;
  adv_den : Z;
  adv_den_pos : 0 < adv_den
}.

(** a ≤ b  iff  a.num × b.den ≤ b.num × a.den  (cross-multiply, sound for q > 0). *)
Definition adv_le (a b : Advantage) : Prop :=
  a.(adv_num) * b.(adv_den) <= b.(adv_num) * a.(adv_den).

(* ---------------------------------------------------------------------------
   Standard 2^128 denominator used throughout QASH security arguments.
   --------------------------------------------------------------------------- *)

Definition two_pow_128 : Z := 2 ^ 128.

Lemma two_pow_128_pos : 0 < two_pow_128.
Proof.
  unfold two_pow_128.
  apply Z.pow_pos_nonneg; lia.
Qed.

(* ---------------------------------------------------------------------------
   Concrete advantage bounds parameterised by query / block count.
   --------------------------------------------------------------------------- *)

(** PRF distinguishing advantage for q oracle queries: q / 2^128. *)
Definition PRF_advantage (q : Z) : Advantage :=
  mkAdvantage q two_pow_128 two_pow_128_pos.

(** Almost-Universal MAC forgery advantage for n message blocks: n / 2^128. *)
Definition AU_MAC_advantage (n : Z) : Advantage :=
  mkAdvantage n two_pow_128 two_pow_128_pos.

(* ---------------------------------------------------------------------------
   Arithmetic lemmas about adv_le.
   --------------------------------------------------------------------------- *)

Lemma adv_le_refl : forall a, adv_le a a.
Proof.
  intros [n d Hd]. unfold adv_le; simpl. lia.
Qed.

Lemma adv_le_trans : forall a b c,
  adv_le a b -> adv_le b c -> adv_le a c.
Proof.
  intros [an ad Had] [bn bd Hbd] [cn cd Hcd].
  unfold adv_le; simpl.
  intros Hab Hbc.
  (* an * bd <= bn * ad  and  bn * cd <= cn * bd *)
  (* Goal: an * cd <= cn * ad *)
  (* an * bd * cd <= bn * ad * cd <= cn * bd * ad *)
  assert (H1 : an * bd * cd <= bn * ad * cd) by (apply Z.mul_le_mono_nonneg_r; lia).
  assert (H2 : bn * cd * ad <= cn * bd * ad) by (apply Z.mul_le_mono_nonneg_r; lia).
  nia.
Qed.

Lemma au_mac_advantage_mono : forall n m,
  n <= m -> adv_le (AU_MAC_advantage n) (AU_MAC_advantage m).
Proof.
  intros n m H.
  unfold adv_le, AU_MAC_advantage; simpl.
  apply Z.mul_le_mono_nonneg_r; [lia | apply Z.lt_le_incl, two_pow_128_pos].
Qed.

Lemma prf_advantage_mono : forall q1 q2,
  q1 <= q2 -> adv_le (PRF_advantage q1) (PRF_advantage q2).
Proof.
  intros q1 q2 H.
  unfold adv_le, PRF_advantage; simpl.
  apply Z.mul_le_mono_nonneg_r; [lia | apply Z.lt_le_incl, two_pow_128_pos].
Qed.
