(** * QASH — Effect-Capability Token Schema (Stage 6a / 1-A)

    File:    proofs/capability/cap_token_schema.v
    Spec:    docs/spec/00_execution_model.md §3 (Domain A / Domain B boundary)
    Class:   FORMAL THEOREM (proof obligations)
    Status:  Proof obligations defined; production proofs pending EffectToken
             migration in advance_epoch signature (tracked separately).

    Theorems proved here
    --------------------
    CT-1  EffectToken wrapping is injective:
            effect_token_wrap_injective —
            EffectToken::new(a) = EffectToken::new(b) -> a = b.

    CT-2  into_inner is the left inverse of new:
            effect_token_roundtrip —
            into_inner(new(x)) = x.

    CT-3  EffectToken cannot be constructed from Domain B output without
          explicitly calling EffectToken::new (structural uniqueness of
          the boundary constructor — modelled as a constructor axiom).

    Background
    ----------
    `EffectToken<T>` wraps Domain B values destined for Domain A.  The type
    has no Default, Clone, or Copy impl, ensuring that every crossing of the
    B→A boundary is explicit and auditable.  CT-1 and CT-2 together prove
    that the wrapping is a lossless bijection — no information is destroyed
    or duplicated at the boundary.

    This file models EffectToken abstractly as an inductive type; it does not
    depend on the Rust implementation directly but captures the invariants that
    the implementation must satisfy.
*)

Require Import Coq.Logic.FunctionalExtensionality.
Require Import Coq.Logic.ProofIrrelevance.

(** Abstract model of EffectToken<T> as a thin wrapper. *)
Inductive EffectToken (T : Type) : Type :=
  | wrap : T -> EffectToken T.

Arguments wrap {T} _.

(** into_inner: consume the token and return the inner value. *)
Definition into_inner {T : Type} (tok : EffectToken T) : T :=
  match tok with
  | wrap v => v
  end.

(** CT-1: Wrapping is injective — two equal tokens imply equal inner values. *)
Lemma effect_token_wrap_injective : forall (T : Type) (a b : T),
    wrap a = wrap b -> a = b.
Proof.
  intros T a b H.
  inversion H.
  reflexivity.
Qed.

(** CT-2: into_inner is the left inverse of wrap (roundtrip). *)
Lemma effect_token_roundtrip : forall (T : Type) (x : T),
    into_inner (wrap x) = x.
Proof.
  intros T x.
  reflexivity.
Qed.

(** CT-3: Every EffectToken was constructed by wrap (constructor uniqueness). *)
Lemma effect_token_constructor_unique : forall (T : Type) (tok : EffectToken T),
    exists v : T, tok = wrap v.
Proof.
  intros T tok.
  destruct tok as [v].
  exists v.
  reflexivity.
Qed.

(** Corollary: into_inner is surjective — every T can appear as the inner value. *)
Lemma effect_token_into_inner_surjective : forall (T : Type) (x : T),
    exists tok : EffectToken T, into_inner tok = x.
Proof.
  intros T x.
  exists (wrap x).
  reflexivity.
Qed.
