(** it_mac_forgery_bound.v

    Purpose:
      Mechanise the numerical part of the IT-MAC security claim used by
      `derive.rs`: for a message of at most 16 blocks, the forging advantage
      is upper-bounded by 16 / 2^128, assuming the standard GHASH-style
      almost-universal polynomial MAC bound.

    Scope:
      The arithmetic cap (n/2^128 <= 16/2^128 for n <= 16) is fully proved.
      The underlying game-based AU property is stated as a typed axiom
      (`ghash_poly_mac_au_bound`) rather than a vacuous `True` placeholder,
      so Coq type-checks the statement structure even though the game proof
      is deferred to SSProve/CryptHOL.
*)

Require Import Coq.ZArith.ZArith.
Require Import Lia.
Require Import QASH.crypto_game_framework.
Open Scope Z_scope.

(* ---------------------------------------------------------------------------
   Abstract forgery advantage.
   Using a Parameter (not a Definition equal to AU_MAC_advantage) ensures
   the axiom below is a genuine constraint, not a trivial adv_le_refl.
   --------------------------------------------------------------------------- *)

(** The actual GHASH forgery advantage for n blocks.  Abstract — its concrete
    value is unknown; the axiom below bounds it from above. *)
Parameter ghash_forgery_advantage : Z -> Advantage.

(* ---------------------------------------------------------------------------
   Arithmetic theorems (fully proved, no axioms).
   --------------------------------------------------------------------------- *)

(** The 16-block cap: 16 * 2^128 is exactly the AU_MAC_advantage 16 numerator. *)
Theorem it_mac_forgery_bound_at_16_blocks :
  forall blocks,
    0 <= blocks <= 16 ->
    (blocks * two_pow_128 <= 16 * two_pow_128)%Z.
Proof.
  intros blocks Hrange.
  apply Z.mul_le_mono_nonneg_r; [apply Z.lt_le_incl, two_pow_128_pos | lia].
Qed.

(* ---------------------------------------------------------------------------
   Typed axiom for the game-based AU property.
   Replace with a proved theorem once the GHASH polynomial MAC is mechanised
   in SSProve or CryptHOL and imported here.
   --------------------------------------------------------------------------- *)

(** ghash_poly_mac_au_bound: the GHASH polynomial MAC is almost-universal.
    For a message of n blocks, the forgery advantage is at most n / 2^128.

    This is a well-typed statement (adv_le between Advantage values), not a
    vacuous True.  It captures the correct mathematical shape: the adversary's
    advantage grows linearly with n and is bounded by the GF(2^128) field size.

    Justification: standard GF(2^128) polynomial MAC security; see
    Bernstein 2005 "The Poly1305-AES message-authentication code" §7. *)
Axiom ghash_poly_mac_au_bound :
  forall n : Z, 0 <= n ->
    adv_le (ghash_forgery_advantage n) (AU_MAC_advantage n).

(* ---------------------------------------------------------------------------
   Derived theorems using the axiom.
   --------------------------------------------------------------------------- *)

(** Instantiate at n = 16: the 8-family cascade IT-MAC forgery bound. *)
Theorem it_mac_forgery_bound_16 :
  adv_le (ghash_forgery_advantage 16) (AU_MAC_advantage 16).
Proof.
  apply ghash_poly_mac_au_bound. lia.
Qed.
