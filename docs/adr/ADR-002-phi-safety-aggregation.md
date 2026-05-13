# ADR-002: Φ_safety Aggregation Rule

- **Status:** proposed
- **PDF anchor:** PDF-SILENT, depends on ADR-001
- **Traceability rows:** P0-1

## Context

The PDF does not define `Φ_safety`, `Φ_max_safe`, or an aggregation rule over
per-validator slash/evidence accumulators.

## Decision options

1. `max_i(Σ_i)` — tracks the worst individual validator.
2. `Σ_i Σ_i` — tracks total system-wide accumulated safety evidence.
3. A bounded or weighted aggregate with explicit cap and proof obligations.

## Proposed decision

Use a sum over validators unless further review shows that the PDF intended
`signature_health` to represent a single global scalar.

## Rationale

A max-only rule can undercount distributed safety degradation. A sum-based rule
better represents aggregate risk, but must include checked arithmetic and a
precise overflow/halt rule.

## Consequences

The current implementation must not be judged compliant or non-compliant until
this ADR is accepted. If the sum rule is accepted, tests must include multiple
validators with nonzero accumulators to distinguish sum from max behavior.
