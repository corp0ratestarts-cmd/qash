# Prior-Art Differentiation: Solana-Family Systems

> Technical working notes for counsel; not a legal prior-art opinion.

## Similarities to review

- High-performance replicated execution.
- Cryptographic state observations.
- Validator participation and replay concerns.

## QASH differentiators to substantiate

- QASH excludes speculative execution from its core model and prioritizes
  replay-stable transition semantics.
- Domain B acceleration is permitted only when it cannot change Domain A
  consensus-visible bytes or transition outcomes.
- Cross-ISA equivalence is framed as an enforcement artifact, not merely a
  performance portability goal.
- Fixed-point consensus arithmetic avoids floating-point or hardware-dependent
  numeric behavior.

## Evidence to collect

- Solana runtime/replay documentation.
- Known deterministic execution constraints in Solana programs and validators.
- Comparison of Solana performance-oriented parallelism with QASH's explicit
  deterministic transition calculus.
