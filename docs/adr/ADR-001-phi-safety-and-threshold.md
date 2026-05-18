# ADR-001 — Φ_safety: aggregation + threshold + halt gate
**Status:** Accepted  
**Filed:** 2026-05-13  
**Depends on:** ERR-001 (Accepted)  
**PDF anchor:** PDF-SILENT (no independent Φ gate or threshold defined)

## Decisions

### D1 — Aggregation
**Chosen:** Sum over validators
`Φ_safety(t) = W_S · Σ_i slash_accumulator_i(t)`

Rationale (non-normative):
- "Σ" suggests aggregation; sum captures total-system accumulation.
- Monotonicity proof is straightforward.

Implementation note:
- Implemented as sum in `lyapunov.rs` (evaluate) and `transition.rs` (evaluate_projected).

### D2 — Threshold parameter
Pinned consensus parameter in `GENESIS_CONSTANTS.toml`:
```toml
[lyapunov]
phi_max_safe = 500_000_000
```
Implemented in `lyapunov.rs` as `PHI_MAX_SAFE: FixedPoint = FixedPoint::from_raw(500_000_000)` and included in `consensus_params_hash()`. With `W_S = 250_000` and `SCALE = 1_000_000`, the boundary is reached when aggregate slash energy is `Σ_i slash_i = 2_000_000_000` raw units.

### D3 — Halt gate
If `Φ_safety(t) >= phi_max_safe` then absorbing halt with a distinct reason.

## Acceptance criteria (becomes CI gate once Accepted)
- Code:
  - `crates/consensus/src/lyapunov.rs` computes Φ as SUM (not max)
  - `crates/consensus/src/transition.rs` applies H7 (`PhiSafetyViolation = 0x07`) deterministically before the commit point
- Vectors:
  - Regression tests include two validators with nonzero slashes and Φ reflecting the sum, plus the `φ == PHI_MAX_SAFE` boundary
- Cross-ISA:
  - Same vectors pass bitwise-identical on x86_64 + aarch64 + riscv64 (via script)
