# ADR 0002: Transition-Safe Fixed-Point Arithmetic

## Status

Accepted.

## Context

Validator stability metrics require fractional quantities, but floating-point
arithmetic can vary by processor, compiler, optimization setting, or instruction
selection. Consensus-visible arithmetic must replay identically.

## Decision

QASH represents fractional protocol values with fixed-point integers. Operations
are checked, rescaled explicitly, and use floor division toward negative
infinity. Arithmetic failures return errors that map to deterministic halt or
rejection behavior instead of wrapping or silently saturating.

## Alternatives considered

- **Floating-point metrics**: rejected because cross-platform replay could
  diverge.
- **Unchecked integer arithmetic**: rejected because overflow behavior would
  create consensus hazards.
- **Saturating arithmetic by default**: rejected because it can hide invalid
  transitions and make failures less observable.

## Consequences

- Stability evaluation is deterministic and proof-friendly.
- Implementations must handle overflow explicitly.
- Benchmarking should measure fixed-point evaluation cost at validator limits.
