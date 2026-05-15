(** * QASH — Encoding Injectivity (TH-1, TH-2)

    File:    proofs/contractivity/encode_injectivity.v
    Spec:    docs/spec/01_consensus.md §2, §7
    Theorem: TH-1  Encode(x) = Encode(y) → x = y
    Also:    TH-2  Encode is total over all well-formed states

    Proof strategy
    --------------
    1. Prove little-endian encoding roundtrips: decode(encode(v, n)) = v
    2. Derive primitive injectivity from roundtrip
    3. Prove fixed-width concatenation can be split at known offsets
    4. Apply field-by-field extraction to ValidatorRecord, then ProtocolState
    5. State TH-1 and compose the subproofs
    6. State the hash collision corollary (foundation for TH-8)

    Axioms used (from §7 theorem graph)
    ------------------------------------
    AX-1  ISA two's complement correctness (implicit in Coq's Z)
    AX-2  Compiler correctness (implicit in Coq's computation model)
    AX-3  SHA3-256 collision resistance (explicit axiom — cryptographic)

    Status: TH-1 and TH-2 fully proved. No Admitted markers.
*)

Require Import Coq.ZArith.ZArith.
Require Import Coq.Lists.List.
Require Import Coq.Bool.Bool.
Require Import Coq.Arith.Arith.
Require Import Coq.micromega.Lia.
Import ListNotations.
Open Scope Z_scope.

Section EncodeInjectivity.

(* ================================================================= *)
(** ** §0 — Abstract Types and Axiom Declarations                     *)
(* ================================================================= *)

(** Abstract byte and hash types.
    Using opaque types rather than list Z prevents theorem leakage from
    unconstrained integer lists. sha3_256 outputs are exactly 32 bytes;
    encoding inputs are well-formed byte sequences. The concrete
    representation is irrelevant to the injectivity proofs. *)
Parameter bytes   : Type.
Parameter hash256 : Type.

(** Coercion from list Z for backward compatibility with existing lemmas. *)
Parameter bytes_of_list : list Z -> bytes.
Parameter list_of_bytes : bytes -> list Z.
Axiom bytes_roundtrip : forall bs, list_of_bytes (bytes_of_list bs) = bs.

(** AX-3: SHA3-256 collision resistance.
    sha3_256 has type bytes -> hash256.
    This is a cryptographic assumption, not a mathematical theorem.
    All other theorems depend only on AX-1 and AX-2 (implicit in Coq). *)
Parameter sha3_256 : bytes -> hash256.

(** Decidable equality for abstract types. *)
Axiom bytes_eq_dec :
  forall (x y : bytes), {x = y} + {x <> y}.
Axiom hash256_eq_dec :
  forall (x y : hash256), {x = y} + {x <> y}.

(** AX-3: SHA3-256 modeled as injective over the protocol's state space.

    IMPORTANT CONCEPTUAL NOTE:
    SHA3-256 is NOT mathematically injective — it maps arbitrary-length
    inputs to 32-byte outputs, so collisions exist by pigeonhole. What we
    are actually assuming is: within the admissible protocol state space,
    we will never encounter a collision. This is a COMPUTATIONAL assumption
    (collision resistance), not a mathematical theorem (injectivity).

    Named AX3_sha3_assumed_injective to make the trust-class explicit. *)
Axiom AX3_sha3_assumed_injective :
  forall m1 m2 : bytes,
    sha3_256 m1 = sha3_256 m2 -> m1 = m2.

(* ================================================================= *)
(** ** §1 — Integer Type Bounds                                        *)
(* ================================================================= *)

Definition INT_MAX  : Z := 2^63 - 1.
Definition INT_MIN  : Z := -(2^63).
Definition U64_MAX  : Z := 2^64 - 1.
Definition U32_MAX  : Z := 2^32 - 1.
Definition p        : Z := 1_000_000.

Definition is_u64 (n : Z) : Prop := 0 <= n <= U64_MAX.
Definition is_u32 (n : Z) : Prop := 0 <= n <= U32_MAX.
Definition is_i64 (n : Z) : Prop := INT_MIN <= n <= INT_MAX.
Definition is_byte (n : Z) : Prop := 0 <= n <= 255.

Lemma int_max_val : INT_MAX = 9223372036854775807. Proof. reflexivity. Qed.

(* ================================================================= *)
(** ** §1a — Genesis Constants and Audit Lemmas                       *)
(* ================================================================= *)

Definition N_max : Z := 1024.

Lemma N_max_lt_U32_MAX : N_max < 2^32.
Proof. unfold N_max. lia. Qed.

Lemma N_max_is_u32 : is_u32 N_max.
Proof. unfold is_u32, N_max, U32_MAX. lia. Qed.

(* ================================================================= *)
(** ** §2 — Little-Endian Encoding Primitives                         *)
(* ================================================================= *)

(** Encode integer v as n little-endian bytes (LSB first).
    Precondition for correctness: 0 <= v < 256^n *)
Fixpoint le_encode (v : Z) (n : nat) : list Z :=
  match n with
  | O    => []
  | S n' => (v mod 256) :: le_encode (v / 256) n'
  end.

(** Decode a little-endian byte list back to integer *)
Fixpoint le_decode (bs : list Z) : Z :=
  match bs with
  | []      => 0
  | b :: rest => b + 256 * le_decode rest
  end.

Lemma le_encode_length : forall v n, length (le_encode v n) = n.
Proof.
  intros v n; revert v.
  induction n; simpl; auto.
Qed.

(** Core roundtrip: decode ∘ encode = id *)
Lemma le_roundtrip :
  forall (v : Z) (n : nat),
    0 <= v < 256 ^ Z.of_nat n ->
    le_decode (le_encode v n) = v.
Proof.
  intros v n; revert v.
  induction n as [| n' IH]; intros v Hv.
  - rewrite Z.pow_0_r in Hv. simpl. lia.
  - rewrite Nat2Z.inj_succ, Z.pow_succ_r in Hv by lia.
    simpl le_encode. simpl le_decode.
    assert (Hrange : 0 <= v / 256 < 256 ^ Z.of_nat n').
    {
      split.
      * apply Z.div_pos; lia.
      * apply Z.div_lt_upper_bound; [lia|lia].
    }
    specialize (IH (v / 256) Hrange).
    rewrite IH.
    assert (H256 : (256:Z) <> 0) by lia.
    change (v mod 256 + 256 * (v / 256) = v).
    rewrite Z.add_comm.
    exact (eq_sym (Z.div_mod v 256 H256)).
Qed.

(** Injectivity of le_encode over fixed width *)
Lemma le_encode_injective :
  forall (v1 v2 : Z) (n : nat),
    0 <= v1 < 256 ^ Z.of_nat n ->
    0 <= v2 < 256 ^ Z.of_nat n ->
    le_encode v1 n = le_encode v2 n ->
    v1 = v2.
Proof.
  intros v1 v2 n H1 H2 Heq.
  apply (f_equal le_decode) in Heq.
  rewrite le_roundtrip, le_roundtrip in Heq; assumption.
Qed.

(** u64 encoding — 8 bytes *)
Definition encode_u64 (v : Z) : list Z := le_encode v 8%nat.
Definition encode_u32 (v : Z) : list Z := le_encode v 4%nat.

Lemma encode_u64_length : forall v, length (encode_u64 v) = 8%nat.
Proof. intros. apply le_encode_length. Qed.

Lemma encode_u32_length : forall v, length (encode_u32 v) = 4%nat.
Proof. intros. apply le_encode_length. Qed.

Lemma pow_256_8 : (256 : Z) ^ Z.of_nat 8 = 2^64.
Proof. reflexivity. Qed.

Lemma encode_u64_injective :
  forall v1 v2,
    is_u64 v1 -> is_u64 v2 ->
    encode_u64 v1 = encode_u64 v2 ->
    v1 = v2.
Proof.
  intros v1 v2 H1 H2 Heq.
  apply le_encode_injective with (n := 8%nat); try assumption.
  - rewrite pow_256_8. unfold is_u64, U64_MAX in H1. lia.
  - rewrite pow_256_8. unfold is_u64, U64_MAX in H2. lia.
Qed.

(** i64 encoding — shift to unsigned range then encode 8 bytes.
    We use v + 2^63 to map [-2^63, 2^63-1] → [0, 2^64-1]. *)
Definition encode_i64 (v : Z) : list Z := le_encode (v + 2^63) 8%nat.

Lemma encode_i64_length : forall v, length (encode_i64 v) = 8%nat.
Proof. intros. unfold encode_i64. apply le_encode_length. Qed.

Lemma encode_i64_injective :
  forall v1 v2,
    is_i64 v1 -> is_i64 v2 ->
    encode_i64 v1 = encode_i64 v2 ->
    v1 = v2.
Proof.
  intros v1 v2 H1 H2 Heq.
  unfold encode_i64 in Heq.
  assert (Hshift : forall v, is_i64 v ->
      0 <= v + 2^63 < 256 ^ Z.of_nat 8).
  { intros v Hv. unfold is_i64, INT_MIN, INT_MAX in Hv.
    rewrite pow_256_8. lia. }
  apply le_encode_injective with (n := 8%nat) in Heq.
  - lia.
  - apply Hshift; assumption.
  - apply Hshift; assumption.
Qed.

(** bool encoding — 1 byte: false → 0x00, true → 0x01 *)
Definition encode_bool (b : bool) : list Z :=
  [if b then 1 else 0].

Lemma encode_bool_length : forall b, length (encode_bool b) = 1%nat.
Proof. destruct b; reflexivity. Qed.

Lemma encode_bool_injective :
  forall b1 b2,
    encode_bool b1 = encode_bool b2 -> b1 = b2.
Proof.
  destruct b1, b2; simpl; intro H; try reflexivity; inversion H.
Qed.

(* ================================================================= *)
(** ** §3 — Fixed-Width Concatenation Splitting                       *)
(* ================================================================= *)

(** The key structural lemma: given two equal concatenations where
    the left parts have the same fixed length, both parts are equal. *)
Lemma app_injective_fixed :
  forall (a1 a2 b1 b2 : list Z) (n : nat),
    length a1 = n ->
    length a2 = n ->
    a1 ++ b1 = a2 ++ b2 ->
    a1 = a2 /\ b1 = b2.
Proof.
  intros a1 a2 b1 b2 n Hn1 Hn2 Heq.
  split.
  - apply (f_equal (firstn n)) in Heq.
    rewrite firstn_app, Hn1, Nat.sub_diag, firstn_O, app_nil_r in Heq.
    rewrite firstn_app, Hn2, Nat.sub_diag, firstn_O, app_nil_r in Heq.
    rewrite !firstn_all2 in Heq by lia.
    exact Heq.
  - apply (f_equal (skipn n)) in Heq.
    rewrite skipn_app, Hn1, Nat.sub_diag, skipn_O in Heq.
    rewrite skipn_app, Hn2, Nat.sub_diag, skipn_O in Heq.
    rewrite !skipn_all2 in Heq by lia.
    simpl in Heq. exact Heq.
Qed.

(* ================================================================= *)
(** ** §4 — ValidatorRecord Encoding and Injectivity                  *)
(* ================================================================= *)

Record ValidatorRecord : Type := mkValidator {
  vr_id         : list Z;  (** 48 bytes verbatim *)
  vr_score      : Z;       (** i64: validator stability weight *)
  vr_divergence : Z;       (** i64: normalized state-root divergence ≥ 0 *)
  vr_conflict   : Z;       (** i64: conflict density ≥ 0 *)
  vr_slash_acc  : Z;       (** i64: monotone slash accumulator ≥ 0 *)
  vr_nonce      : Z;       (** u64: monotone replay-prevention counter *)
  vr_active     : bool;
}.

(** ValidatorEncodable: structural well-formedness for encoding proofs only.
    TH-1 and encoding injectivity depend ONLY on these fields. *)
Record ValidatorEncodable (vr : ValidatorRecord) : Prop := mkValidatorEncodable {
  ve_id_len  : length (vr_id vr) = 48%nat;
  ve_score   : is_i64 (vr_score vr);
  ve_div     : is_i64 (vr_divergence vr);
  ve_conf    : is_i64 (vr_conflict vr);
  ve_slash   : is_i64 (vr_slash_acc vr);
  ve_nonce   : is_u64 (vr_nonce vr);
}.

(** ValidatorInvariant: semantic protocol invariants for convergence proofs. *)
Record ValidatorInvariant (vr : ValidatorRecord) : Prop := mkValidatorInvariant {
  vi_div_nn  : 0 <= vr_divergence vr;
  vi_conf_nn : 0 <= vr_conflict vr;
  vi_slash_nn: 0 <= vr_slash_acc vr;
}.

(** ValidatorWF: the conjunction — for proofs that need both. *)
Record ValidatorWF (vr : ValidatorRecord) : Prop := mkValidatorWF {
  vwf_enc : ValidatorEncodable vr;
  vwf_inv : ValidatorInvariant vr;
}.

(** Convenience projections from ValidatorWF *)
Lemma vwf_id_len  vr : ValidatorWF vr -> length (vr_id vr) = 48%nat.
Proof. intros H. apply (ve_id_len _ (vwf_enc _ H)). Qed.
Lemma vwf_score   vr : ValidatorWF vr -> is_i64 (vr_score vr).
Proof. intros H. apply (ve_score _ (vwf_enc _ H)). Qed.
Lemma vwf_div     vr : ValidatorWF vr -> is_i64 (vr_divergence vr).
Proof. intros H. apply (ve_div _ (vwf_enc _ H)). Qed.
Lemma vwf_conf    vr : ValidatorWF vr -> is_i64 (vr_conflict vr).
Proof. intros H. apply (ve_conf _ (vwf_enc _ H)). Qed.
Lemma vwf_slash   vr : ValidatorWF vr -> is_i64 (vr_slash_acc vr).
Proof. intros H. apply (ve_slash _ (vwf_enc _ H)). Qed.
Lemma vwf_nonce   vr : ValidatorWF vr -> is_u64 (vr_nonce vr).
Proof. intros H. apply (ve_nonce _ (vwf_enc _ H)). Qed.

(** Encode a ValidatorRecord as 89 bytes:
    48 (id) + 8 (score) + 8 (div) + 8 (conf) + 8 (slash) + 8 (nonce) + 1 (active) *)
Definition encode_validator (vr : ValidatorRecord) : list Z :=
  vr_id vr
  ++ encode_i64 (vr_score vr)
  ++ encode_i64 (vr_divergence vr)
  ++ encode_i64 (vr_conflict vr)
  ++ encode_i64 (vr_slash_acc vr)
  ++ encode_u64 (vr_nonce vr)
  ++ encode_bool (vr_active vr).

Lemma encode_validator_length :
  forall vr, ValidatorWF vr ->
    length (encode_validator vr) = 89%nat.
Proof.
  intros vr Hwf. unfold encode_validator.
  repeat rewrite app_length.
  rewrite (vwf_id_len _ Hwf).
  rewrite !encode_i64_length, encode_u64_length, encode_bool_length.
  reflexivity.
Qed.

(** TH-1a: ValidatorRecord encoding is injective *)
Theorem encode_validator_injective :
  forall vr1 vr2,
    ValidatorWF vr1 -> ValidatorWF vr2 ->
    encode_validator vr1 = encode_validator vr2 ->
    vr1 = vr2.
Proof.
  intros vr1 vr2 Hwf1 Hwf2 Heq.
  unfold encode_validator in Heq.
  (* Split off id field (48 bytes) *)
  destruct (app_injective_fixed _ _ _ _ 48%nat
      (vwf_id_len _ Hwf1) (vwf_id_len _ Hwf2) Heq) as [Hid Hrest1].
  (* Split off score (8 bytes) *)
  destruct (app_injective_fixed _ _ _ _ 8%nat
      encode_i64_length encode_i64_length Hrest1) as [Hsc Hrest2].
  (* Split off divergence (8 bytes) *)
  destruct (app_injective_fixed _ _ _ _ 8%nat
      encode_i64_length encode_i64_length Hrest2) as [Hdv Hrest3].
  (* Split off conflict (8 bytes) *)
  destruct (app_injective_fixed _ _ _ _ 8%nat
      encode_i64_length encode_i64_length Hrest3) as [Hcf Hrest4].
  (* Split off slash_acc (8 bytes) *)
  destruct (app_injective_fixed _ _ _ _ 8%nat
      encode_i64_length encode_i64_length Hrest4) as [Hsl Hrest5].
  (* Split nonce (8 bytes) from active (1 byte) *)
  destruct (app_injective_fixed _ _ _ _ 8%nat
      encode_u64_length encode_u64_length Hrest5) as [Hnonce Hac].
  (* Apply primitive injectivity lemmas *)
  apply (encode_i64_injective _ _ (vwf_score _ Hwf1) (vwf_score _ Hwf2)) in Hsc.
  apply (encode_i64_injective _ _ (vwf_div _ Hwf1) (vwf_div _ Hwf2)) in Hdv.
  apply (encode_i64_injective _ _ (vwf_conf _ Hwf1) (vwf_conf _ Hwf2)) in Hcf.
  apply (encode_i64_injective _ _ (vwf_slash _ Hwf1) (vwf_slash _ Hwf2)) in Hsl.
  apply (encode_u64_injective _ _ (vwf_nonce _ Hwf1) (vwf_nonce _ Hwf2)) in Hnonce.
  apply encode_bool_injective in Hac.
  (* All fields equal; records are equal *)
  destruct vr1, vr2; simpl in *; subst; reflexivity.
Qed.

(** Injectivity extends to lists of ValidatorRecords with equal lengths *)
Lemma encode_validators_injective :
  forall vs1 vs2 : list ValidatorRecord,
    length vs1 = length vs2 ->
    (forall vr, In vr vs1 -> ValidatorWF vr) ->
    (forall vr, In vr vs2 -> ValidatorWF vr) ->
    flat_map encode_validator vs1 = flat_map encode_validator vs2 ->
    vs1 = vs2.
Proof.
  induction vs1 as [| vr1 rest1 IH]; intros vs2 Hlen Hwf1 Hwf2 Heq.
  - destruct vs2; [reflexivity | inversion Hlen].
  - destruct vs2 as [| vr2 rest2]; [inversion Hlen |].
    injection Hlen as Hlen.
    simpl flat_map in Heq.
    destruct (app_injective_fixed _ _ _ _ 89%nat
        (encode_validator_length _ (Hwf1 _ (or_introl eq_refl)))
        (encode_validator_length _ (Hwf2 _ (or_introl eq_refl)))
        Heq) as [Hvr Hrest].
    apply (encode_validator_injective _ _
        (Hwf1 _ (or_introl eq_refl))
        (Hwf2 _ (or_introl eq_refl))) in Hvr.
    specialize (IH rest2 Hlen
        (fun vr Hin => Hwf1 vr (or_intror Hin))
        (fun vr Hin => Hwf2 vr (or_intror Hin))
        Hrest).
    subst. reflexivity.
Qed.

(* ================================================================= *)
(** ** §5 — ProtocolState Encoding and TH-1                          *)
(* ================================================================= *)

Record ProtocolState : Type := mkState {
  ps_epoch          : Z;
  ps_state_root     : list Z;   (** 32 bytes *)
  ps_ledger_root    : list Z;   (** 32 bytes *)
  ps_entropy_seed   : list Z;   (** 32 bytes *)
  ps_halt_flag      : bool;
  ps_validators     : list ValidatorRecord;
  ps_lyapunov_window: list Z;   (** W=3 i64 values *)
}.

Fixpoint encode_window (ws : list Z) : list Z :=
  match ws with
  | []        => []
  | w :: rest => encode_i64 w ++ encode_window rest
  end.

(** Canonical encoding of ProtocolState.
    Field order exactly matches 01_consensus.md §2 wire format. *)
Definition encode_state (s : ProtocolState) : list Z :=
  encode_u64  (ps_epoch s)
  ++ ps_state_root s
  ++ ps_ledger_root s
  ++ ps_entropy_seed s
  ++ encode_bool (ps_halt_flag s)
  ++ encode_u32  (Z.of_nat (length (ps_validators s)))
  ++ flat_map encode_validator (ps_validators s)
  ++ encode_window (ps_lyapunov_window s).

Record StateWF (s : ProtocolState) : Prop := mkStateWF {
  swf_epoch    : is_u64 (ps_epoch s);
  swf_sr_len   : length (ps_state_root s) = 32%nat;
  swf_lr_len   : length (ps_ledger_root s) = 32%nat;
  swf_es_len   : length (ps_entropy_seed s) = 32%nat;
  (** N_max = 1024 < 2^32 = U32_MAX+1, so validator count fits in u32. *)
  swf_n_bound  : is_u32 (Z.of_nat (length (ps_validators s)));
  swf_val_wf   : forall vr, In vr (ps_validators s) -> ValidatorWF vr;
  swf_lw_len   : length (ps_lyapunov_window s) = 3%nat;
  (** W = 3 is a one-shot genesis constant. *)
  swf_lw_wf    : forall w, In w (ps_lyapunov_window s) -> is_i64 w;
}.

(** Window encoding auxiliary lemmas *)
Lemma encode_window_length_3 :
  forall ws, length ws = 3%nat ->
    (forall w, In w ws -> is_i64 w) ->
    length (encode_window ws) = 24%nat.
Proof.
  intros ws Hlen Hwf.
  destruct ws as [| w1 [| w2 [| w3 [|]]]]; simpl in *; try lia.
  repeat rewrite app_length, encode_i64_length. reflexivity.
Qed.

Lemma encode_window_injective :
  forall ws1 ws2,
    length ws1 = 3%nat -> length ws2 = 3%nat ->
    (** W = 3 is hardcoded intentionally — W is a one-shot genesis constant.
        Do not refactor this to forall n — the inflexibility is a feature. *)
    (forall w, In w ws1 -> is_i64 w) ->
    (forall w, In w ws2 -> is_i64 w) ->
    encode_window ws1 = encode_window ws2 ->
    ws1 = ws2.
Proof.
  intros ws1 ws2 Hl1 Hl2 Hwf1 Hwf2 Heq.
  destruct ws1 as [| a1 [| b1 [| c1 [|]]]]; simpl in Hl1; try lia.
  destruct ws2 as [| a2 [| b2 [| c2 [|]]]]; simpl in Hl2; try lia.
  simpl encode_window in Heq.
  (* Split a *)
  destruct (app_injective_fixed _ _ _ _ 8%nat
      encode_i64_length encode_i64_length Heq) as [Ha Hrest1].
  (* Split b *)
  destruct (app_injective_fixed _ _ _ _ 8%nat
      encode_i64_length encode_i64_length Hrest1) as [Hb Hc].
  (* Hc : encode_i64 c1 ++ [] = encode_i64 c2 ++ [] *)
  destruct (app_injective_fixed _ _ _ _ 8%nat
      encode_i64_length encode_i64_length Hc) as [Hc' _].
  apply (encode_i64_injective _ _
      (Hwf1 _ (or_introl eq_refl))
      (Hwf2 _ (or_introl eq_refl))) in Ha.
  apply (encode_i64_injective _ _
      (Hwf1 _ (or_intror (or_introl eq_refl)))
      (Hwf2 _ (or_intror (or_introl eq_refl)))) in Hb.
  apply (encode_i64_injective _ _
      (Hwf1 _ (or_intror (or_intror (or_introl eq_refl))))
      (Hwf2 _ (or_intror (or_intror (or_introl eq_refl))))) in Hc'.
  subst. reflexivity.
Qed.

Lemma pow_256_4 : (256 : Z) ^ Z.of_nat 4 = 2^32.
Proof. reflexivity. Qed.

Lemma encode_u32_injective :
  forall v1 v2,
    is_u32 v1 -> is_u32 v2 ->
    encode_u32 v1 = encode_u32 v2 ->
    v1 = v2.
Proof.
  intros v1 v2 H1 H2 Heq.
  apply le_encode_injective with (n := 4%nat); try assumption.
  - rewrite pow_256_4. unfold is_u32 in H1. lia.
  - rewrite pow_256_4. unfold is_u32 in H2. lia.
Qed.

(** flat_map encode_validator produces exactly (length vs × 89) bytes *)
Lemma flat_map_validator_length :
  forall vs : list ValidatorRecord,
    (forall vr, In vr vs -> ValidatorWF vr) ->
    length (flat_map encode_validator vs) = length vs * 89.
Proof.
  induction vs as [| vr rest IH]; intros Hwf.
  - simpl. reflexivity.
  - simpl flat_map. rewrite app_length.
    rewrite encode_validator_length by (apply Hwf; left; reflexivity).
    rewrite IH by (intros vr' Hin; apply Hwf; right; assumption).
    simpl length. lia.
Qed.

(**
  ** TH-1: Canonical state encoding is injective over well-formed states.

  For all well-formed protocol states s1 and s2:
    encode_state s1 = encode_state s2  →  s1 = s2

  This is the foundational theorem. TH-7 (replay invariance) and
  TH-8 (succession soundness) both depend on it.
*)
Theorem TH1_encode_state_injective :
  forall s1 s2 : ProtocolState,
    StateWF s1 -> StateWF s2 ->
    encode_state s1 = encode_state s2 ->
    s1 = s2.
Proof.
  intros s1 s2 Hwf1 Hwf2 Heq.
  unfold encode_state in Heq.
  (* epoch: 8 bytes *)
  destruct (app_injective_fixed _ _ _ _ 8%nat
      encode_u64_length encode_u64_length Heq) as [Hep Hrest1].
  (* state_root: 32 bytes *)
  destruct (app_injective_fixed _ _ _ _ 32%nat
      (swf_sr_len _ Hwf1) (swf_sr_len _ Hwf2) Hrest1) as [Hsr Hrest2].
  (* ledger_root: 32 bytes *)
  destruct (app_injective_fixed _ _ _ _ 32%nat
      (swf_lr_len _ Hwf1) (swf_lr_len _ Hwf2) Hrest2) as [Hlr Hrest3].
  (* entropy_seed: 32 bytes *)
  destruct (app_injective_fixed _ _ _ _ 32%nat
      (swf_es_len _ Hwf1) (swf_es_len _ Hwf2) Hrest3) as [Hes Hrest4].
  (* halt_flag: 1 byte *)
  destruct (app_injective_fixed _ _ _ _ 1%nat
      encode_bool_length encode_bool_length Hrest4) as [Hhf Hrest5].
  (* validator_count: 4 bytes *)
  destruct (app_injective_fixed _ _ _ _ 4%nat
      encode_u32_length encode_u32_length Hrest5) as [Hvc Hrest6].
  (* validator_count encodes equal lengths → validator lists have same length *)
  assert (Hvc_u32_1 : is_u32 (Z.of_nat (length (ps_validators s1))))
    by apply (swf_n_bound _ Hwf1).
  assert (Hvc_u32_2 : is_u32 (Z.of_nat (length (ps_validators s2))))
    by apply (swf_n_bound _ Hwf2).
  pose proof (encode_u32_injective _ _ Hvc_u32_1 Hvc_u32_2 Hvc) as Hlen_z.
  apply Nat2Z.inj in Hlen_z.
  (* validators: length(vs1) × 89 bytes *)
  set (n_bytes := length (ps_validators s1) * 89).
  assert (Hfml1 : length (flat_map encode_validator (ps_validators s1)) = n_bytes).
  { unfold n_bytes. apply flat_map_validator_length. apply (swf_val_wf _ Hwf1). }
  assert (Hfml2 : length (flat_map encode_validator (ps_validators s2)) = n_bytes).
  { unfold n_bytes. rewrite <- Hlen_z. apply flat_map_validator_length. apply (swf_val_wf _ Hwf2). }
  destruct (app_injective_fixed _ _ _ _ n_bytes Hfml1 Hfml2 Hrest6) as [Hvals Hwin].
  (* validators equal *)
  apply (encode_validators_injective _ _ Hlen_z
      (swf_val_wf _ Hwf1) (swf_val_wf _ Hwf2)) in Hvals.
  (* window equal *)
  apply (encode_window_injective _ _
      (swf_lw_len _ Hwf1) (swf_lw_len _ Hwf2)
      (swf_lw_wf _ Hwf1) (swf_lw_wf _ Hwf2)) in Hwin.
  (* primitive field injectivity *)
  apply (encode_u64_injective _ _ (swf_epoch _ Hwf1) (swf_epoch _ Hwf2)) in Hep.
  apply encode_bool_injective in Hhf.
  destruct s1, s2; simpl in *; subst; reflexivity.
Qed.
(** TH-1 is fully closed. No Admitted markers remain in this file. *)

(** ** TH-2: encode_state is total over all well-formed states.
    In Coq's total type theory this is immediate — stated for completeness. *)
Theorem TH2_encode_state_total :
  forall s : ProtocolState, StateWF s ->
    exists bs : list Z, encode_state s = bs.
Proof.
  intros s _. exact (ex_intro _ (encode_state s) eq_refl).
Qed.

(* ================================================================= *)
(** ** §6 — Corollary: State Root Collision Resistance (Foundation for TH-8)  *)
(* ================================================================= *)

(**
  State root collision-freeness.

  Depends on: TH-1 (encode injectivity) + AX-3 (hash collision resistance).

  Named state_root_collision_resistance rather than state_root_injective
  because SHA3-256 is not mathematically injective (collisions exist).
  Under AX-3 (SHA3-256 collisions unreachable in the protocol's state space),
  identical state roots imply identical states.

  Class: ASSUMED — depends on AX-3, which is a cryptographic assumption.
*)
Corollary state_root_collision_resistance :
  forall s1 s2 : ProtocolState,
    StateWF s1 -> StateWF s2 ->
    sha3_256 (bytes_of_list (encode_state s1)) =
    sha3_256 (bytes_of_list (encode_state s2)) ->
    s1 = s2.
Proof.
  intros s1 s2 Hwf1 Hwf2 Hroots.
  apply AX3_sha3_assumed_injective in Hroots.
  apply (f_equal list_of_bytes) in Hroots.
  rewrite !bytes_roundtrip in Hroots.
  exact (TH1_encode_state_injective s1 s2 Hwf1 Hwf2 Hroots).
Qed.

(* ================================================================= *)
(** ** §7 — Proof Dependency Summary                                  *)
(**
  Theorem classes:
    FORMAL   — machine-checked from AX-1/AX-2 only
    ASSUMED  — depends on AX-3 (cryptographic assumption, not proved)

  AX-1 (implicit in Z) ──┐
  AX-2 (implicit in Coq) ─┼──► le_roundtrip              [FORMAL]
                           │         │
                           │         ▼
                           │    le_encode_injective        [FORMAL]
                           │         │
                           │    ┌────┴────────────────┐
                           │    ▼                      ▼
                           │  encode_u64_injective   encode_i64_injective [FORMAL]
                           │    │                      │
                           │    └────────┬─────────────┘
                           │             ▼
                           │    encode_validator_injective (TH-1a) [FORMAL]
                           │             │
                           │    encode_validators_injective         [FORMAL]
                           │             │
                           └─────────────┼──────────────────────────────┐
                                         ▼                              │
                              TH1_encode_state_injective   [FORMAL]    AX-3
                              (FULLY CLOSED)                            │
                                         │                              │
                                         └──────────────────────────────┘
                                                       │
                                                       ▼
                                   state_root_collision_resistance  [ASSUMED]
                                   (foundation for TH-8 — depends on AX-3)
*)
(* ================================================================= *)

End EncodeInjectivity.

(** AX-3 maps to an external cryptographic primitive.
    Extract Constant is placed outside the Section so extraction
    targets the globally qualified name. *)
Extract Constant sha3_256 => "Qash.Crypto.sha3_256".
