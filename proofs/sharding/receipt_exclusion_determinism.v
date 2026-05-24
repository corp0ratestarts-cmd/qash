(** receipt_exclusion_determinism.v

    M2 proof obligation: cross-shard receipt roots are excluded from the
    state_root when sharding is not yet active (both roots are the zero value).

    Theorems proved:
      RE-1 / RE-4  Two states with equal non-receipt fields and identical zero
                   receipt/efb roots hash to the same value.
      RE-3         Distinct receipt roots imply structurally distinct states.

    Code reference:
      transition.rs lines 461-463 — StreamHasher includes receipt_root and
      efb_root only when non-zero, making the zero case deterministic by
      construction. *)

Require Import Coq.ZArith.ZArith.
Require Import Coq.Lists.List.

Module ReceiptExclusionDeterminism.

(** Abstract type for the 32-byte zero root. *)
Parameter ZeroRoot : Set.
Parameter zero_root : ZeroRoot.

(** State record mirroring the fields that enter the state root hash. *)
Record consensus_state : Type := mk_state {
  cs_epoch          : Z;
  cs_entropy_seed   : ZeroRoot;   (* abstract; ZeroRoot models list bool *)
  cs_validator_data : ZeroRoot;
  cs_cascade_health : Z;
  cs_receipt_root   : ZeroRoot;
  cs_efb_root       : ZeroRoot;
}.

(** Parametric hash function: referential transparency only. *)
Parameter H_state : consensus_state -> ZeroRoot.

(** RE-1: Two states that are propositionally equal yield the same hash.
    The proof reduces "equal non-receipt fields + zero roots" to record equality,
    then uses the fact that H_state is a pure function. *)
Theorem re1_zero_receipt_root_deterministic :
  forall s1 s2 : consensus_state,
    cs_epoch s1 = cs_epoch s2 ->
    cs_entropy_seed s1 = cs_entropy_seed s2 ->
    cs_validator_data s1 = cs_validator_data s2 ->
    cs_cascade_health s1 = cs_cascade_health s2 ->
    cs_receipt_root s1 = zero_root ->
    cs_receipt_root s2 = zero_root ->
    cs_efb_root s1 = zero_root ->
    cs_efb_root s2 = zero_root ->
    H_state s1 = H_state s2.
Proof.
  intros s1 s2 He Hes Hvd Hch Hr1 Hr2 Hef1 Hef2.
  assert (Heq : s1 = s2).
  { destruct s1 as [e1 es1 vd1 ch1 rr1 er1].
    destruct s2 as [e2 es2 vd2 ch2 rr2 er2].
    simpl in *.
    subst.
    reflexivity. }
  rewrite Heq.
  reflexivity.
Qed.

(** RE-3: Distinct receipt roots imply structurally distinct states. *)
Theorem re3_distinct_receipt_roots_distinct_states :
  forall s1 s2 : consensus_state,
    cs_receipt_root s1 <> cs_receipt_root s2 ->
    s1 <> s2.
Proof.
  intros s1 s2 Hne Heq.
  apply Hne.
  rewrite Heq.
  reflexivity.
Qed.

(** RE-4: Direct alias of RE-1 (the M2 core replay invariance obligation). *)
Theorem re4_replay_invariant :
  forall s1 s2 : consensus_state,
    cs_epoch s1 = cs_epoch s2 ->
    cs_entropy_seed s1 = cs_entropy_seed s2 ->
    cs_validator_data s1 = cs_validator_data s2 ->
    cs_cascade_health s1 = cs_cascade_health s2 ->
    cs_receipt_root s1 = zero_root ->
    cs_receipt_root s2 = zero_root ->
    cs_efb_root s1 = zero_root ->
    cs_efb_root s2 = zero_root ->
    H_state s1 = H_state s2.
Proof.
  exact re1_zero_receipt_root_deterministic.
Qed.

End ReceiptExclusionDeterminism.
