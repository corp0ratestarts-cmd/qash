# Proof Status

## Mechanically Verified (CI-gated, no Admitted)

These files are compiled by `make all` in CI and must remain Admitted-free:

| ID | Name | File | Status |
|----|------|------|--------|
| TH-3a | No halt when δ ≤ ε | `contractivity/lyapunov_stability.v` | ✅ PROVED |
| TH-3b | Halt iff δ > ε | `contractivity/lyapunov_stability.v` | ✅ PROVED |
| TH-3c | FinalizeEpoch → V_convergence = 0 | `contractivity/lyapunov_stability.v` | ✅ PROVED |
| TH-9 | CH_t ∈ [0,p], χ·CH_t no overflow | `cascade/cascade_health_bounded.v` | ✅ PROVED |
| — | List encoding infrastructure | `util/list_inj.v` | ✅ PROVED |

**Also compiled (no proof obligations):**

| File | Status |
|------|--------|
| `cascade/cascade_collision_resistance.v` | `Axiom` — cryptographic assumption (TH-10), not `Admitted` |
| `cascade/cascade_determinism.v` | Verification claim (TH-11) — CI-tested, no Coq proof by design |
| `concat_injective.v` | Stub (TBD) |
| `lyapunov_decrease.v` | Stub (TBD) |

## Sketch Drafts (in `_wip/`, NOT compiled by CI)

The Coq files in `_wip/` are design sketches. They capture correct proof
strategy but use invalid Coq syntax (`apply ... by X by Y`, Z/nat scope-mixing)
and have not been mechanically verified by `coqc`.

| ID | Name | Draft file | Issue |
|----|------|------------|-------|
| TH-1 | Encoding injectivity | `_wip/encode_injectivity.v.draft` | Invalid tactic syntax |
| TH-2 | Encoding totality | (depends on TH-1) | Blocked by TH-1 |
| TH-4 | Φ_safety monotonicity | `_wip/absorbing_halt.v.draft` | Invalid tactic syntax |
| TH-5 | Φ_safety boundedness | `_wip/absorbing_halt.v.draft` | Invalid tactic syntax |
| TH-6 | Halt correctness | `_wip/absorbing_halt.v.draft` | Invalid tactic syntax |
| TH-8 | Succession soundness | (depends on TH-1) | Blocked by TH-1 |

## TH-7: Replay Invariance

VERIFIED by CI test suite (golden vector runner + `golden_replay.rs`).
Not formally proved in Coq — the replay invariance guarantee comes from the
deterministic state machine with absorbing halt and the golden hash vectors.

## Genesis Lock Requirement

For genesis-lock, the following are required:
- ✅ TH-3 (Lyapunov stability) — proved
- ✅ TH-9 (Cascade health boundedness) — proved
- TH-1/2/4/5/6/8 — remain as post-genesis obligations or must be discharged
  before final sign-off (see ADR-001 and docs/release/rc_checklist_pack.md)
