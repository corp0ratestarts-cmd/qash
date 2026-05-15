# ADR-001: Φ_safety Accumulator Definition and Halt Gate

- **Status:** proposed
- **Depends on:** ERR-001 accepted two-function partition
- **PDF anchor:** PDF-SILENT, related to §4.1 and ERR-001
- **Traceability rows:** P0-1, P0-9

## Context

ERR-001 partitions the PDF's `L(t)` expression into `V_convergence(t)` and
`Φ_safety(t)`. The PDF does not define `Φ_safety`, `Φ_max_safe`, or a second halt
condition.

## Decisions required

### 1. Aggregation

- **Sum:** `Φ_safety = W_S · Σ_i(slash_i)`. Captures total system risk.
- **Max:** `Φ_safety = W_S · max_i(slash_i)`. Captures worst individual risk.
- **Recommendation:** Sum. The PDF's parameter name uses `Σ`, which
  conventionally denotes summation, and distributed small penalties should not
  disappear behind a max-only rule.

### 2. Threshold

`Φ_max_safe` must be pinned in `GENESIS_CONSTANTS.toml`, or this ADR must define
a deterministic derivation from existing genesis constants.

### 3. Halt behavior

If accepted, `Φ_safety(t) ≥ Φ_max_safe` triggers absorbing halt alongside the
`V_convergence` δ-window gate.

## Non-goals

- This ADR does not define how slash evidence enters the system.
- This ADR does not replace transaction semantics or input admissibility rules.
- This ADR does not claim that the current max-based implementation is
  compliant.

## Impact on traceability

P0-1 can be validated only after this ADR and ADR-002 settle aggregation,
threshold, and halt semantics. P0-9 cannot lock genesis until the resulting
parameter set is frozen.
