# ADR-001 — Φ_safety: aggregation + threshold + halt gate
**Status:** Proposed  
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
- Current code uses `max`; must change to sum if this ADR is accepted.

### D2 — Threshold parameter
Add new genesis constant:
```toml
[lyapunov]
phi_max_safe = ???   # REQUIRED before genesis lock
```

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
