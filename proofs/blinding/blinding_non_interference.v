(** blinding_non_interference.v — quantitative PRF-style bound skeleton. *)

Require Import Coq.QArith.QArith.
Require Import Coq.ZArith.ZArith.
Require Import Lia.
Open Scope Z_scope.

Parameter BlindingKey : Type.
Parameter Message : Type.
Parameter Observable : Type.

Parameter blind_cascade : BlindingKey -> Message -> Observable.

(* Quantitative advantage model (rational in [0,1]). *)
Definition Prob := Q.
Parameter prf_advantage : BlindingKey -> Message -> Message -> Prob.
Parameter noninterference_advantage : BlindingKey -> Message -> Message -> Prob.

(* Phase-1 budget: conservative symbolic bound carried in proofs/docs. *)
Definition prf_budget_num : Z := 1.
Definition prf_budget_den : Z := 2 ^ 128.
Definition prf_budget : Q := Qmake prf_budget_num prf_budget_den.

Axiom prf_budget_den_pos : 0 < prf_budget_den.

(* Reduction hypothesis: non-interference is bounded by PRF distinguishing adv. *)
Axiom noninterference_le_prf :
  forall k m1 m2,
    noninterference_advantage k m1 m2 <= prf_advantage k m1 m2.

(* Quantitative PRF security hypothesis (to be discharged in SSProve/CryptHOL). *)
Axiom prf_advantage_bound :
  forall k m1 m2,
    prf_advantage k m1 m2 <= prf_budget.

Theorem cascade_prf_quantitative_bound :
  forall k m1 m2,
    noninterference_advantage k m1 m2 <= prf_budget.
Proof.
  intros k m1 m2.
  eapply Qle_trans.
  - apply noninterference_le_prf.
  - apply prf_advantage_bound.
Qed.
