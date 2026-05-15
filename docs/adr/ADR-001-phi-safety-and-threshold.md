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
Derived constant (not in TOML; computed from genesis constants):
```
PHI_MAX_SAFE = N_max × floor(γ_raw × i64::MAX / p) / 2
             = 1024 × floor(200_000 × 9_223_372_036_854_775_807 / 1_000_000) / 2
             ≈ 9.44 × 10^20
```
Implemented in `lyapunov.rs` as `PHI_MAX_SAFE: i128`.

### D3 — Halt gate
If `Φ_safety(t) >= phi_max_safe` then absorbing halt with a distinct reason.

## Acceptance criteria (becomes CI gate once Accepted)
- Code:
  - `crates/consensus/src/lyapunov.rs` computes Φ as SUM (not max)
  - `crates/consensus/src/transition.rs` applies H2 deterministically
- Vectors:
  - At least 2 vectors where two validators have nonzero slashes and Φ reflects the sum
- Cross-ISA:
  - Same vectors pass bitwise-identical on x86_64 + aarch64 + riscv64 (via script)
