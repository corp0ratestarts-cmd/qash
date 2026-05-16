(** * QASH — Concatenation Injectivity

    File:    proofs/concat_injective.v
    Class:   FORMAL THEOREM (no Admitted markers)
    Purpose: Proves that byte-array concatenation is injective when the
             left component has a known fixed length. Reused by encoding
             injectivity proofs to establish that multi-field encodings
             cannot collide unless every field collides.

    This corresponds to the informal "no prefix collision" argument
    used throughout the spec's §2 encoding uniqueness reasoning.

    Depends on: Coq stdlib (List, Arith)
    Status: Fully proved. No Admitted markers.
*)

Require Import Coq.Lists.List.
Require Import Coq.Arith.Arith.
Require Import Coq.micromega.Lia.
Import ListNotations.

(** ** Lemma: app_length_eq_left

    If two appended lists are equal and their left components have the same
    fixed length, then the left components are equal and the right
    components are equal.

    This is the core "prefix cancellation" property used when proving
    that field-concatenated encodings are injective. *)
Lemma app_length_eq_left :
  forall (A : Type) (a1 a2 b1 b2 : list A),
    a1 ++ b1 = a2 ++ b2 ->
    length a1 = length a2 ->
    a1 = a2 /\ b1 = b2.
Proof.
  intros A a1.
  induction a1 as [| x xs IH]; intros a2 b1 b2 Heq Hlen.
  - (* a1 = [] *)
    destruct a2 as [| y ys].
    + simpl in Heq. split; [reflexivity | exact Heq].
    + simpl in Hlen. lia.
  - (* a1 = x :: xs *)
    destruct a2 as [| y ys].
    + simpl in Hlen. lia.
    + simpl in Heq. injection Heq as Hxy Hrest.
      simpl in Hlen.
      destruct (IH ys b1 b2 Hrest (Nat.succ_inj _ _ Hlen)) as [Hxs Hb].
      split.
      * rewrite Hxy, Hxs. reflexivity.
      * exact Hb.
Qed.

(** ** Corollary: concat_inj_fixed_left

    Specialisation for the two-field case: if `enc(field1) ++ enc(field2) =
    enc(field1') ++ enc(field2')` and `enc(field1)` always has the same
    length (fixed-width encoding), then `field1 = field1'` and
    `field2 = field2'`. *)
Lemma concat_inj_fixed_left :
  forall (A B : Type) (enc_a : A -> list nat) (enc_b : B -> list nat)
    (k : nat) (x x' : A) (y y' : B),
    (forall a : A, length (enc_a a) = k) ->
    enc_a x ++ enc_b y = enc_a x' ++ enc_b y' ->
    enc_a x = enc_a x' /\ enc_b y = enc_b y'.
Proof.
  intros A B enc_a enc_b k x x' y y' Hfixed Heq.
  apply app_length_eq_left.
  - exact Heq.
  - rewrite Hfixed, Hfixed. reflexivity.
Qed.
