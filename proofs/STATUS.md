# Proof Status

## Mechanically Verified (CI-gated, no Admitted)

These files are compiled by `make all` in CI and must remain Admitted-free:

| ID | Name | File | Status |
|----|------|------|--------|
| TH-3a | No halt when δ ≤ ε | `contractivity/lyapunov_stability.v` | ✅ PROVED |
| TH-3b | Halt iff δ > ε | `contractivity/lyapunov_stability.v` | ✅ PROVED |
| TH-3c | FinalizeEpoch → V_convergence = 0 | `contractivity/lyapunov_stability.v` | ✅ PROVED |
| TH-9 | CH_t ∈ [0,p], χ·CH_t no overflow | `cascade/cascade_health_bounded.v` | ✅ PROVED |
| TH-GC | Grace convergence: ε_honest-bounded steps → δ_window ≤ 3×ε_honest < ε_halt → no H1 halt | `contractivity/lyapunov_grace_convergence.v` | ✅ PROVED |
| TH-1 | Encoding injectivity: encode_state(s1) = encode_state(s2) → s1 = s2 | `encoding_injectivity.v` | ✅ PROVED |
| TH-2 | Encoding totality: encode_state total over well-formed states | `encoding_injectivity.v` | ✅ PROVED |
| TH-4 | Φ_safety monotonicity: Φ_safety(T(S,I)) ≥ Φ_safety(S) | `safety/absorbing_halt.v` | ✅ PROVED |
| TH-5 | Φ_safety boundedness: Φ_safety(S) ≤ Φ_max for all admissible S | `safety/absorbing_halt.v` | ✅ PROVED |
| TH-6 | Halt correctness: halt_flag=true → no admissible transitions | `safety/absorbing_halt.v` | ✅ PROVED |
| TH-8 | Succession soundness (partial): halted state frozen, root unique via TH-1+AX-3 | `safety/absorbing_halt.v` | ✅ PROVED (partial) |
| ERR-001 | V_convergence partition: not monotone, can reach 0; distinct from Φ_safety | `lyapunov_decrease.v` | ✅ PROVED |
| — | List encoding infrastructure | `util/list_inj.v` | ✅ PROVED |

**Also compiled (no proof obligations):**

| File | Status |
|------|--------|
| `cascade/cascade_collision_resistance.v` | `Axiom` — cryptographic assumption (TH-10), not `Admitted` |
| `cascade/cascade_determinism.v` | Verification claim (TH-11) — CI-tested, no Coq proof by design |
| `blinding/blinding_non_interference.v` | `Axiom` — PRF security of H_cascade_keyed (§3.7.5); full proof deferred post-genesis |
| `concat_injective.v` | Stub (TBD) |

## Sketch Drafts (in `_wip/`, NOT compiled by CI)

The Coq files in `_wip/` are historical design sketches. The valid proofs have
been promoted to real files with syntax fixed; originals kept for audit trail.

| ID | Original Draft | Promoted To | Status |
|----|---------------|-------------|--------|
| TH-1/2 | `_wip/encode_injectivity.v.draft` | `encoding_injectivity.v` | ✅ Promoted |
| TH-4/5/6/8 | `_wip/absorbing_halt.v.draft` | `safety/absorbing_halt.v` | ✅ Promoted |

## TH-7: Replay Invariance

VERIFIED by CI test suite (golden vector runner + `golden_replay.rs`).
Not formally proved in Coq — the replay invariance guarantee comes from the
deterministic state machine with absorbing halt and the golden hash vectors.

## Genesis Lock Requirement

For genesis-lock, the following are required:
- ✅ TH-3 (Lyapunov stability) — proved
- ✅ TH-9 (Cascade health boundedness) — proved
- ✅ TH-GC (Grace convergence / tolerance pillar) — proved
- ✅ TH-1/TH-2 (Encoding injectivity / totality) — proved
- ✅ TH-4/TH-5/TH-6 (Φ_safety monotonicity, boundedness, halt terminal) — proved
- ✅ TH-8 (Succession soundness partial) — proved; full composition deferred post-genesis
- ✅ ERR-001 (V_convergence partition) — proved

All pre-genesis proof obligations discharged. See ADR-001 for Φ_safety threshold
and ADR-004 for halt layering (Domain A vs PAL).
