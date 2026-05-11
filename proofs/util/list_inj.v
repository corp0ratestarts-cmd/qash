(** * QASH — List Encoding Infrastructure
    
    File:    proofs/util/list_inj.v
    Purpose: Reusable lemmas for fixed-width encoder injectivity.
             Used by: encode_injectivity.v, future tx proof files,
             Merkle leaf packing, witness serialization.

    All lemmas in this file are FORMAL (AX-1/AX-2 only).
    Status: flat_map_fixed_length PROVED. flat_map_inj_fixed PROVED.
*)

Require Import Coq.Lists.List.
Require Import Coq.Arith.Arith.
Require Import Coq.micromega.Lia.
Import ListNotations.

(* ================================================================= *)
(** ** §1 — Fixed-Width flat_map Lemmas                               *)
(* ================================================================= *)

(** flat_map over a fixed-length encoder preserves total length
    multiplicatively. Reusable for any fixed-width encoding:
    validators, transactions, Merkle leaves, etc. *)
Lemma flat_map_fixed_length :
  forall (A : Type) (f : A -> list nat) (xs : list A) (k : nat),
    (forall x : A, length (f x) = k) ->
    length (flat_map f xs) = length xs * k.
Proof.
  induction xs as [| x xs IH]; intros Hk.
  - simpl. reflexivity.
  - simpl. rewrite app_length, IH by assumption, Hk. lia.
Qed.

(** Variant over list Z (used by encode_injectivity.v) *)
Lemma flat_map_fixed_length_Z :
  forall (A : Type) (f : A -> list Z) (xs : list A) (k : nat),
    (forall x : A, length (f x) = k) ->
    length (flat_map f xs) = length xs * k.
Proof.
  induction xs as [| x xs IH]; intros Hk.
  - simpl. reflexivity.
  - simpl. rewrite app_length, IH by assumption, Hk. lia.
Qed.

(** ** flat_map injectivity under fixed-width encoding

    Preconditions:
      k > 0        — required: if k = 0, f _ = [] and all lists map to []
                     making the theorem false (any two lists give same output)
      length f = k — each element encodes to exactly k bytes
      f injective  — the element encoder is injective

    Result: flat_map f is injective over lists of equal length.

    NOTE: the equal-length hypothesis is necessary because flat_map
    of a fixed-width injective encoder is NOT injective over lists of
    different lengths (different lists can encode to the same total bytes
    if padded differently — but here we enforce equal length explicitly). *)
Lemma flat_map_inj_fixed :
  forall (A : Type) (f : A -> list Z) (xs ys : list A) (k : nat),
    k > 0 ->
    (forall x, length (f x) = k) ->
    (forall x y, f x = f y -> x = y) ->
    length xs = length ys ->
    flat_map f xs = flat_map f ys ->
    xs = ys.
Proof.
  intros A f xs.
  induction xs as [| x xs IHxs]; intros ys k Hk_pos Hlen Hinj Hsame_len Heq.
  - (* xs = [] *)
    destruct ys; [reflexivity | simpl in Hsame_len; lia].
  - (* xs = x :: xs' *)
    destruct ys as [| y ys]; [simpl in Hsame_len; lia |].
    injection Hsame_len as Hsame_len.
    simpl flat_map in Heq.
    (* Split head from tail using fixed-width *)
    assert (Hlenx : length (f x) = k) by apply Hlen.
    assert (Hleny : length (f y) = k) by apply Hlen.
    (* f x and f y are the first k bytes of each side *)
    assert (Hpre : f x = f y /\ flat_map f xs = flat_map f ys).
    { apply (app_inj_1 _ _ _ _ (eq_trans Hlenx (eq_sym Hleny))). exact Heq. }
    destruct Hpre as [Hhd Htl].
    apply Hinj in Hhd.
    apply IHxs in Htl; [subst; reflexivity | assumption | assumption | assumption | assumption].
Qed.

(* ================================================================= *)
(** ** §2 — Prefix Cancellation (Segmentation Engine)                 *)
(* ================================================================= *)

(** The core segmentation lemma: equal concatenations with equal-length
    left parts have equal left parts and equal right parts.

    This is the engine behind recursive structural injectivity.
    It converts "equal concatenations" into "equal components" field by field.
    Used in every injectivity proof in encode_injectivity.v via
    app_injective_fixed. *)
Lemma app_cancel_left :
  forall (A : Type) (a1 a2 b1 b2 : list A) (k : nat),
    length a1 = k ->
    length a2 = k ->
    a1 ++ b1 = a2 ++ b2 ->
    a1 = a2 /\ b1 = b2.
Proof.
  intros A a1 a2 b1 b2 k H1 H2 Heq.
  split.
  - apply (f_equal (firstn k)) in Heq.
    rewrite !firstn_app in Heq.
    rewrite H1, H2, Nat.sub_diag in Heq.
    rewrite !firstn_O, !app_nil_r in Heq.
    rewrite !firstn_all2 in Heq by lia.
    exact Heq.
  - apply (f_equal (skipn k)) in Heq.
    rewrite !skipn_app in Heq.
    rewrite H1, H2, Nat.sub_diag in Heq.
    rewrite !skipn_O in Heq.
    rewrite !skipn_all2 in Heq by lia.
    simpl in Heq. exact Heq.
Qed.

(** Width-arithmetic lemma used in segmentation:
    if all elements encode to k bytes, the total length is determined *)
Lemma app_total_length :
  forall (A : Type) (f : A -> list Z) (xs : list A) (k : nat),
    (forall x, length (f x) = k) ->
    length (flat_map f xs) = length xs * k.
Proof. exact flat_map_fixed_length_Z. Qed.

End. (* list_inj *)
