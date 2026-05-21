Require Import Coq.Lists.List.
Require Import Coq.ZArith.ZArith.
Import ListNotations.
Open Scope Z_scope.

Module EfbDeterminism.

Record shard_commitment := {
  shard_id : Z;
  state_root : list bool;
  receipt_root : list bool
}.

Record efb_input := {
  epoch : Z;
  previous_efb_root : list bool;
  shard_commitments : list shard_commitment;
  zk_batch_root : list bool
}.

Parameter H_efb : list bool -> list bool.
Parameter encode_efb_input : efb_input -> list bool.
Parameter receipt_epoch : list bool -> Z.
Parameter receipt_source_shard : list bool -> Z.
Parameter receipt_target_shard : list bool -> Z.
Parameter efb_epoch : list bool -> Z.
Parameter efb_shard_count : list bool -> Z.

Definition compute_efb_root (i : efb_input) : list bool :=
  H_efb (encode_efb_input i).

Definition receipt_epoch_anchored (receipt efb : list bool) : Prop :=
  receipt_epoch receipt = efb_epoch efb /\
  0 <= receipt_source_shard receipt < efb_shard_count efb /\
  0 <= receipt_target_shard receipt < efb_shard_count efb.

Theorem efb_root_deterministic :
  forall i, compute_efb_root i = compute_efb_root i.
Proof.
  intros. reflexivity.
Qed.

Theorem same_efb_input_same_root :
  forall a b,
    a = b ->
    compute_efb_root a = compute_efb_root b.
Proof.
  intros a b H. subst. reflexivity.
Qed.

Theorem receipt_epoch_mismatch_rejected :
  forall receipt efb,
    receipt_epoch receipt <> efb_epoch efb ->
    ~ receipt_epoch_anchored receipt efb.
Proof.
  intros receipt efb Hneq Hanchored.
  destruct Hanchored as [Heq _].
  contradiction.
Qed.

End EfbDeterminism.
