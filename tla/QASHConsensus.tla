---- MODULE QASHConsensus ----
EXTENDS Naturals
(*
  STUB: exists to validate Apalache wiring.
  Replace with real model per PDF §9.2.
*)
VARIABLES epoch
Init == epoch = 0
Next == epoch' = epoch + 1
Spec == Init /\ [][Next]_<<epoch>>
Invariant == epoch >= 0
====
