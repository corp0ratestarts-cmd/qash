# ADR-002: Φ_safety Aggregation Rule

- **Status:** accepted
- **PDF anchor:** PDF-SILENT, depends on ADR-001
- **Traceability rows:** P0-1

## Context

The PDF does not define `Φ_safety`, `Φ_max_safe`, or an aggregation rule over
per-validator slash/evidence accumulators.

## Decision

**Use sum aggregation:** `Φ_safety = W_S · Σ_i(slash_i)`.

This is consistent with ADR-001's decision. A max-only rule undercounts
distributed safety degradation: 100 validators each with a moderate slash look
the same as 1 validator with the same slash under max, but represent
qualitatively different system risk. Sum-based aggregation captures total
system-wide accumulated safety evidence and is checked arithmetic throughout
(overflow → H2 ArithOverflow halt).

## Rationale for rejecting max

- `max_i(slash_i)` tracks only the single worst validator; the protocol has no
  mechanism to redistribute or clear slash accumulators, so distributed
  accumulation is the expected failure mode.
- The PDF's parameter notation (`Σ`) is summation, not maximum.

## Consequences

- The implementation in `lyapunov.rs` and `transition.rs` uses `checked_add`
  over `slash_accum` fields rather than `max`.
- Tests exist that distinguish sum from max: two validators each with slash
  `400_000_000` raw produce `phi_safety = 200_000_000` (sum), which is double
  what max would produce (`100_000_000`).
- P0-1 compliance can now be assessed; status advances to ⚠️ (tests pass,
  proof CI not yet verified).
