(* Causal ordering determinism and injectivity skeleton for v1.1. *)
From Coq Require Import List.
From Coq Require Import NArith.
Import ListNotations.

Module CausalOrdering.

Definition byte := N.
Definition bytes := list byte.

Record envelope := {
  env_epoch : N;
  env_sort_key : bytes;
}.

Definition envelope_le (a b : envelope) : Prop :=
  (env_epoch a < env_epoch b)%N \/
  (env_epoch a = env_epoch b /\ env_sort_key a <= env_sort_key b).

Parameter H_domain : N -> bytes -> bytes.
Definition causal_tag : N := 32%N.

Definition causal_preimage (epoch_seed : bytes) (shard_id : N) (envelope_hash : bytes) : bytes :=
  epoch_seed ++ [shard_id] ++ envelope_hash.

Definition compute_sort_key (epoch_seed : bytes) (shard_id : N) (envelope_hash : bytes) : bytes :=
  H_domain causal_tag (causal_preimage epoch_seed shard_id envelope_hash).

Theorem compute_sort_key_deterministic :
  forall seed sid h,
    compute_sort_key seed sid h = compute_sort_key seed sid h.
Proof. reflexivity. Qed.

Axiom H_domain_injective :
  forall tag x y,
    H_domain tag x = H_domain tag y -> x = y.

Theorem compute_sort_key_injective :
  forall seed sid h seed' sid' h',
    compute_sort_key seed sid h = compute_sort_key seed' sid' h' ->
    causal_preimage seed sid h = causal_preimage seed' sid' h'.
Proof.
  intros.
  unfold compute_sort_key in H.
  eapply H_domain_injective in H.
  exact H.
Qed.

End CausalOrdering.
