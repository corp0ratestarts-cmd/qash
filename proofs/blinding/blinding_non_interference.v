(** blinding_non_interference.v — Non-interference theorem skeleton (spec §3.7.5).

    Status: AXIOM — the non-interference property is a cryptographic assumption
    based on the PRF security of H_cascade_keyed.  A full proof would require
    formalising the PRF game in Coq (CryptHOL / SSProve style) which is
    deferred to the post-genesis proof obligation list.

    Theorem (informal, §3.7.5):
      For any blinded operation with valid blinding_params and any two secrets
      s₁, s₂:
        Observations(exec(s₁)) ≈_c Observations(exec(s₂))
      i.e. under computational indistinguishability, even a full side-channel
      observer cannot distinguish executions on different secrets.

    This is entailed by:
      1. PRF security of H_cascade_keyed  [assumed: cascade_prf_security below]
      2. Additive masking correctness: blind(x) = H_cascade_keyed(k, x) ⊕ x
         is computationally indistinguishable from random given secret k.
      3. Dilithium blinding soundness: multiplicative scalar from step 2 does
         not leak the message [deferred — hardware-specific side channel].
*)

Require Import Coq.Strings.String.

(* ---------------------------------------------------------------------------
   Abstract types
   --------------------------------------------------------------------------- *)

Parameter BlindingKey : Type.
Parameter Message     : Type.
Parameter Observable  : Type.

(* The blinded cascade operation. *)
Parameter blind_cascade : BlindingKey -> Message -> Observable.

(* Two observations are computationally indistinguishable. *)
Parameter computationally_indistinguishable : Observable -> Observable -> Prop.

(* ---------------------------------------------------------------------------
   Cryptographic axiom (PRF security of H_cascade_keyed)
   --------------------------------------------------------------------------- *)

(** cascade_prf_security: H_cascade_keyed(k, ·) is a PRF family.
    Any two messages produce outputs that are computationally indistinguishable
    from each other when k is unknown to the adversary. *)
Axiom cascade_prf_security :
  forall (k : BlindingKey) (m1 m2 : Message),
    computationally_indistinguishable
      (blind_cascade k m1)
      (blind_cascade k m2).

(* ---------------------------------------------------------------------------
   Non-interference theorem
   Follows directly from cascade_prf_security.
   --------------------------------------------------------------------------- *)

Theorem blinding_non_interference :
  forall (k : BlindingKey) (s1 s2 : Message),
    computationally_indistinguishable
      (blind_cascade k s1)
      (blind_cascade k s2).
Proof.
  intros k s1 s2.
  exact (cascade_prf_security k s1 s2).
Qed.
