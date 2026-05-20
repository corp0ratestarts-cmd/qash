(*
  v1.1 causal ordering proof module.

  Scope:
  - Determinism of sort-key computation from identical inputs.
  - Injectivity of the preimage under a hash injectivity assumption.
  - Basic ordering lemmas for epoch/lexicographic key ordering.

  This file is intentionally lightweight and CI-compilable, with explicit
  assumptions documented as axioms (no Admitted).
*)
From Coq Require Import List.
From Coq Require Import NArith.
From Coq Require Import Relations.
From Coq Require Import Bool.Bool.
Import ListNotations.

Module CausalOrdering.

Definition byte := N.
Definition bytes := list byte.

Record envelope := {
  env_epoch : N;
  env_sort_key : bytes;
}.

(* Lexicographic order induced from N order on bytes. *)
Definition byte_lt (a b : byte) : bool := N.ltb a b.
Definition byte_eq (a b : byte) : bool := N.eqb a b.

Fixpoint lex_lt (xs ys : bytes) : bool :=
  match xs, ys with
  | [], [] => false
  | [], _ :: _ => true
  | _ :: _, [] => false
  | x :: xt, y :: yt =>
      if byte_lt x y then true
      else if byte_eq x y then lex_lt xt yt
      else false
  end.

Definition envelope_lt (a b : envelope) : Prop :=
  (env_epoch a < env_epoch b)%N \/
  (env_epoch a = env_epoch b /\ lex_lt (env_sort_key a) (env_sort_key b) = true).

Parameter H_domain : N -> bytes -> bytes.
Definition causal_tag : N := 32%N.

Definition causal_preimage
  (epoch_seed : bytes)
  (shard_id : N)
  (envelope_hash : bytes)
  : bytes :=
  epoch_seed ++ [shard_id] ++ envelope_hash.

Definition compute_sort_key
  (epoch_seed : bytes)
  (shard_id : N)
  (envelope_hash : bytes)
  : bytes :=
  H_domain causal_tag (causal_preimage epoch_seed shard_id envelope_hash).

Theorem compute_sort_key_deterministic :
  forall seed sid h,
    compute_sort_key seed sid h = compute_sort_key seed sid h.
Proof. reflexivity. Qed.

Axiom H_domain_injective :
  forall tag x y,
    H_domain tag x = H_domain tag y -> x = y.

Theorem compute_sort_key_injective_preimage :
  forall seed sid h seed' sid' h',
    compute_sort_key seed sid h = compute_sort_key seed' sid' h' ->
    causal_preimage seed sid h = causal_preimage seed' sid' h'.
Proof.
  intros.
  unfold compute_sort_key in H.
  eapply H_domain_injective in H.
  exact H.
Qed.

Theorem lex_lt_irreflexive :
  forall xs,
    lex_lt xs xs = false.
Proof.
  induction xs as [|x xt IH]; simpl; auto.
  rewrite N.ltb_irrefl.
  rewrite N.eqb_refl.
  exact IH.
Qed.

Theorem envelope_lt_irreflexive :
  forall e,
    ~ envelope_lt e e.
Proof.
  intros e [Hepoch | [Heq Hlex]].
  - apply (N.lt_irrefl (env_epoch e)); exact Hepoch.
  - apply lex_lt_irreflexive in Hlex. discriminate.
Qed.

End CausalOrdering.
