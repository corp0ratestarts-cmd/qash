# Prior-Art Differentiation: Ethereum-Family Systems

> Technical working notes for counsel; not a legal prior-art opinion.

## Similarities to review

- Replicated state execution.
- Canonical transaction/state encoding.
- Validator or consensus participant accountability in proof-of-stake variants.
- Deterministic execution expectations for clients.

## QASH differentiators to substantiate

- Protocol-law separation between deterministic Domain A and nondeterministic
  Domain B, with cross-domain contamination defined as nonconformance.
- Lyapunov-style validator convergence scoring using deterministic fixed-point
  arithmetic and a replay-visible halt path.
- Runtime/model observational equivalence as an explicit proof target rather
  than only independent client compatibility.
- Cross-ISA replay evidence and state-root equivalence artifacts as first-class
  repository deliverables.

## Evidence to collect

- Ethereum client determinism requirements and execution-spec references.
- Any known Ethereum formal verification or consensus test suites that resemble
  QASH's runtime/model equivalence target.
- Differences between Ethereum slashing/inactivity handling and QASH's
  convergence-window halt semantics.
