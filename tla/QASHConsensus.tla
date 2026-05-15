---- MODULE QASHConsensus ----
EXTENDS Naturals, Sequences

\* STUB: replace with the full consensus model required by PDF §9.2.
\* This module exists to exercise the Apalache CI pipeline with a non-trivial
\* invariant while the executable model and full transition relation are built.

VARIABLES epoch, lyapunov_value

MAX_LYAPUNOV == 20000

Init == /\ epoch = 0
        /\ lyapunov_value = 0

Next == /\ epoch' = epoch + 1
        /\ lyapunov_value' \in 0..MAX_LYAPUNOV

Spec == Init /\ [][Next]_<<epoch, lyapunov_value>>

SafetyInvariant == lyapunov_value <= MAX_LYAPUNOV

====
