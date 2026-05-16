# ADR-001: Φ_safety Accumulator Definition and Halt Gate

- **Status:** accepted
- **Depends on:** ERR-001 accepted two-function partition
- **PDF anchor:** PDF-SILENT, related to §4.1 and ERR-001
- **Traceability rows:** P0-1, P0-9

## Context

ERR-001 partitions the PDF's `L(t)` expression into `V_convergence(t)` and
`Φ_safety(t)`. The PDF does not define `Φ_safety`, `Φ_max_safe`, or a second halt
condition.

## Decisions

### 1. Aggregation

**Decision: Sum.** `Φ_safety = W_S · Σ_i(slash_i)`.

The PDF parameter name uses `Σ`, which conventionally denotes summation.
A max-only rule suppresses distributed safety degradation: if 100 validators
each accumulate a moderate slash, a max rule sees only one of them. Sum
captures total system risk and is consistent with `V_convergence` being a sum
over all validators.

### 2. Threshold

**Decision: `PHI_MAX_SAFE = 500_000_000` raw i128 units**, pinned in
`GENESIS_CONSTANTS.toml` as `phi_max_safe = 500_000_000`.

Derivation: with `W_S = 0.25` (raw 250_000) and `SCALE = 1_000_000`, this
threshold is reached when the aggregate slash energy `Σ slash_i ≥ 2_000_000_000`
raw units — equivalent to 2000 SCALE-units of accumulated slash across all
validators. For a 1024-validator network this means an average of ~1.95
full-scale slashes per validator, which is a severe degradation event.

### 3. Halt behavior

**Decision: `Φ_safety(t) ≥ PHI_MAX_SAFE` triggers H7 (`PhiSafetyViolation`)
absorbing halt**, evaluated before the commit point alongside the H1
(`LyapunovViolation`) δ-window gate. H1 and H7 are independent gates; either
is sufficient to halt.

## Implementation

- `crates/consensus/src/lyapunov.rs`: `PHI_MAX_SAFE` constant; `evaluate()` uses
  `checked_add` for `sum_slash`; `LyapunovEval.phi_halt_triggered` set when
  `phi.raw() >= PHI_MAX_SAFE.raw()`.
- `crates/consensus/src/transition.rs`: `HaltReason::PhiSafetyViolation = 0x07`;
  `evaluate_projected()` mirrors the sum logic; `run_pipeline()` checks
  `phi_halt_triggered` before the commit point.
- `GENESIS_CONSTANTS.toml`: `phi_max_safe = 500_000_000` in `[lyapunov]`.
- `crates/consensus/src/params.rs`: `PHI_MAX_SAFE` included in params fingerprint.

## Non-goals

- This ADR does not define how slash evidence enters the system.
- This ADR does not replace transaction semantics or input admissibility rules.

## Impact on traceability

P0-1 advances from 🔶 to ⚠️: ADR-001 and ADR-002 are resolved; the implementation
is validated by tests that distinguish sum from max. Remaining gap: Lyapunov proof
is not CI-verified. P0-9 remains ❌ until the normative PDF is committed and all
encoding is defined.
