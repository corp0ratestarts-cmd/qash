(** crypto_game_framework.v

    Lightweight Coq scaffold for game-based crypto proofs used by
    Phase-1 obligations (TH-10, Blinding PRF, IT-MAC).

    This module intentionally defines the interfaces and advantage notions
    without claiming concrete security proofs yet. It is the bridge from
    current axioms to future SSProve/CryptHOL-style mechanisation.
*)

Require Import Coq.ZArith.ZArith.
Require Import Coq.QArith.QArith.
Require Import Lia.
Open Scope Z_scope.

Module Type FINITE_TYPES.
  Parameter Key Msg Tag OracleOut : Type.
End FINITE_TYPES.

Module CryptoGame (T : FINITE_TYPES).
  Import T.

  (** Probability values represented as rationals in [0,1]. *)
  Definition Prob := Q.

  Parameter prob_wf : Prob -> Prop.
  Axiom prob_wf_0 : prob_wf 0.
  Axiom prob_wf_1 : prob_wf 1.

  (** Generic oracle interface (small-step style). *)
  Parameter OracleState : Type.
  Parameter Oracle : Type.
  Parameter query : Oracle -> OracleState -> Msg -> (OracleOut * OracleState).

  (** Adversary is abstract: can adaptively query oracle and output a bit. *)
  Parameter Adversary : Type.
  Parameter run_adversary : Adversary -> Oracle -> Prob.

  (** Distinguishing advantage template. *)
  Definition advantage (a : Adversary) (real ideal : Oracle) : Prob :=
    (run_adversary a real - run_adversary a ideal)%Q.

  (** PRF game skeleton. *)
  Parameter keyed_oracle : Key -> Oracle.
  Parameter random_oracle : Oracle.

  Definition prf_advantage (a : Adversary) (k : Key) : Prob :=
    advantage a (keyed_oracle k) random_oracle.

  (** AU-MAC forgery game skeleton. *)
  Parameter forge_advantage : Adversary -> Key -> Z -> Prob.

  (** Standard target shape: Adv_forge <= n / 2^128. *)
  Definition two_pow_128 : Z := 2 ^ 128.
  Definition forge_cap (blocks : Z) : Q := Qmake blocks two_pow_128.

  Axiom two_pow_128_pos : 0 < two_pow_128.

  (** Transition lemma placeholder:
      if per-block AU bound is established, phase cap follows by monotonicity. *)
  Theorem forge_cap_monotone :
    forall n m,
      0 <= n <= m ->
      (n * two_pow_128 <= m * two_pow_128)%Z.
  Proof.
    intros n m Hnm.
    assert (Hle : n <= m) by lia.
    apply Z.mul_le_mono_nonneg_r; try lia.
    apply Z.lt_le_incl, two_pow_128_pos.
  Qed.

End CryptoGame.
