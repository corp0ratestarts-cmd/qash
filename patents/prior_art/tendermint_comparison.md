# Prior-Art Differentiation: Tendermint / CometBFT-Family Systems

> Technical working notes for counsel; not a legal prior-art opinion.

## Similarities to review

- Validator-set based consensus.
- Deterministic application state machine replication.
- Byzantine fault tolerance and accountability concepts.

## QASH differentiators to substantiate

- Deterministic replay isolation is stated as protocol law that invalidates
  implementations when Domain B values influence Domain A transitions.
- Validator stability is evaluated with fixed-point Lyapunov-style convergence
  metrics before updates commit.
- Absorbing halt behavior is tied to deterministic arithmetic, decoding, and
  convergence-window failures.
- Claim-support mapping ties proposed claim elements directly to source files,
  proofs, diagrams, and replay traces.

## Evidence to collect

- Tendermint application/block execution determinism requirements.
- Validator update and evidence handling mechanisms.
- Any prior art on halting state transitions based on deterministic validator
  convergence functions.
