# QASH Security Model

**Document:** `docs/spec/11_security_model.md`  
**Spec version:** v1.1.1  
**Status:** NORMATIVE

---

## §11.1 Threat Surface

QASH's threat model covers four classes of adversarial input:

| Class | Description | Primary Defence |
|---|---|---|
| **Byzantine validators** | Validators that submit conflicting or invalid state transitions | Lyapunov convergence gate (H1); slash accumulator (Φ_safety gate H7) |
| **Arithmetic exploitation** | Attempts to overflow i128 consensus arithmetic | Checked arithmetic throughout Domain A; overflow → absorbing halt (H2) |
| **Cross-ISA divergence** | Subtle differences in fixed-point arithmetic output across authorised ISAs | Domain A determinism rules (no float, no usize, checked arithmetic); TH-GC tolerance margin |
| **Side-channel extraction** | Power/timing/EM attacks on key material | Blinding framework (§3.7); TH-SB non-interference axiom |

The threat model does **not** cover:
- Network-level censorship (liveness is out of scope for this document)
- Social engineering or governance attacks (QASH is zero-governance by design)
- Physical access to validator hardware below the PAL abstraction boundary

---

## §11.2 Domain Partition Security (Domain A / Domain B)

The fundamental security boundary is the Domain A / Domain B partition defined in `00_execution_model.md §E0`.

**Domain A (Deterministic Consensus)**  
All code in `qash-consensus`. Rules: no `unsafe`, no `f32`/`f64`, no `usize`/`isize`, no wall-clock, no entropy ingress, all arithmetic checked. Replay-invariant across all authorised ISAs.

**Domain B (PAL / Operational)**  
All code in `qash-pal` and the hosted binary. Nondeterminism and `unsafe` permitted; `Halt::absorbing_reset()` is the only escape path into Domain A.

**Contamination rule:** A Domain B value influencing a Domain A computation is a protocol violation even if tests pass. The compiler enforces this via the `no_std`/`forbid(unsafe_code)` boundary on `qash-consensus`.

---

## §11.3 Absorbing Halt Containment

QASH's halt mechanism is designed for **local containment**: a single validator halting does not affect the canonical chain.

**Halt semantics (from `00_execution_model.md §E5`):**

| Code | Condition | Scope |
|---|---|---|
| H1 | `δ_window > ε_halt` | Validator-local |
| H2 | i128 arithmetic overflow | Validator-local |
| H3 | u64 epoch counter overflow | Validator-local |
| H4 | Decode / admissibility failure | Validator-local |
| H5 | State root round-trip failure | Validator-local (reserved) |
| H6 | Explicit external halt flag | Validator-local (reserved) |
| H7 | `Φ_safety ≥ Φ_max_safe` | Network-wide (ADR-001) |

**Containment invariant:** For H1–H6, `advance_epoch()` returns `Err(halt_reason)` and freezes all state except `halt_reason`. The halt is irreversible and produces no broadcast. The network treats the halted node as offline and continues with the remaining validator set.

H7 (Φ_safety gate) is the sole network-wide halt condition: it is triggered only when the aggregate slash evidence across **all** validators exceeds the `Φ_max_safe` threshold derived from `i64::MAX`.

---

## §11.4 Clone Protocol Bounded Reconciliation

Detached or temporarily offline nodes reconcile via the clone protocol (`src/offline/clone.rs`). Admission is bounded:

- **Maximum hops:** 7 (`CloneHop.hop_index` ∈ {0..6})
- **Maximum epoch offset:** 12 (`CloneHop.epoch_offset` ∈ {0..11})
- **Verification:** each `CascadeBoundCloneChunk` carries a cascade inclusion proof (`verify_l7`) that must verify against the epoch's `cascade_root`

A chunk whose proof fails is discarded. A node that cannot reconcile within the hop/epoch bounds must resync from the canonical tip. This ensures no divergent state can be injected into the live chain via the clone path.

---

## §11.5 Tolerance-Based Divergence Containment (Bat Immunology Model)

QASH treats bounded execution variance (cross-ISA timing differences, transient cryptographic noise, JIT precision drift) as **noise to be absorbed**, not faults to be eliminated. This mirrors bat immunology: tolerance, compartmentalization, and controlled reconciliation — not destructive inflammatory response.

### Formal Mapping

| Bat Biological Principle | QASH Technical Equivalent | Proof / Guarantee |
|---|---|---|
| **Tolerance over elimination** | 3-epoch `ConvergenceWindow` + ε_honest/ε_halt 10× margin | TH-GC: bounded honest steps never trigger H1 halt |
| **Compartmentalization** | Local absorbing halt (H1–H6); network continues | §11.3 containment invariant |
| **Immune filtering** | Tiered ISA attestation (Tier A/B/C, `00_execution_model.md §E0`) | TH-7: replay invariance across Tier A ISAs |
| **Self-healing reconciliation** | Clone protocol bounded admission (≤7 hops, ≤12 epochs) | §11.4 bounded reconciliation |
| **Non-destructive response** | Blinding framework (PRF masking, chunk-key sharing) | TH-SB: non-interference axiom (`blinding/blinding_non_interference.v`) |

### Tolerance Detection Equation

The **tolerance margin remaining** is a deterministic, Domain A observable computed at the end of every epoch evaluation:

```
tolerance_margin_remaining(t) = max(0,  ε_halt − δ_window(t))
                               = max(0,  20_000 − (V_convergence(t) − min(preceding_window)))
```

This value is carried in `LyapunovEval.tolerance_margin_remaining` and may be observed by Domain B monitoring without influencing consensus.

- `tolerance_margin_remaining > 0`: network is in the tolerance zone; no halt imminent.
- `tolerance_margin_remaining = 0`: the current epoch's δ_window has reached or exceeded ε_halt; H1 triggers.

### Formal Guarantee: TH-GC (Grace Convergence)

**Theorem TH-GC** (proved in `proofs/contractivity/lyapunov_grace_convergence.v`):

> If for every epoch t in the evaluation window, the per-epoch change in V_convergence satisfies  
> `|V(t) − V(t−1)| ≤ ε_honest = 2_000`,  
> then `δ_window(t) ≤ WINDOW_SIZE × ε_honest = 3 × 2_000 = 6_000 < ε_halt = 20_000`,  
> and therefore `halt_triggered(t) = false`.

**Corollary:** Under honest operation, `tolerance_margin_remaining(t) ≥ ε_halt − 3×ε_honest = 14_000 > 0`. The tolerance margin is never exhausted by honest behaviour alone.

**Proof sketch:** The 3-epoch window stores the three preceding V_convergence values. In the worst case (monotone increasing), V grows by at most ε_honest each epoch, so the oldest window value is at most 3×ε_honest below the current value. The min of the window cannot be smaller than the oldest value, so δ_window ≤ 3×ε_honest. Since 3×ε_honest = 6_000 ≤ 20_000 = ε_halt, TH-3a applies and halt_triggered = false.

### Why Cross-ISA Divergence Does Not Split the Network

Cross-ISA divergence cannot produce a network fork under QASH's architecture:

1. **Halt is local (H1–H6).** A validator whose V_convergence diverges beyond ε_halt enters absorbing halt and drops offline. The canonical chain continues with the remaining set. There is no "split" — one chain continues, one node halts.

2. **δ_window is epoch-bounded.** A single-epoch ISA precision artifact perturbs V_convergence by at most `α·D_max + β·C_max + χ·CH_max` per validator. For this to exceed ε_halt, the artifact would have to move every metric to its maximum simultaneously for every validator — a physically impossible condition for benign ISA variance.

3. **Domain A determinism is ISA-independent by construction.** Fixed-point arithmetic over `i128` with no `usize`, no floats, and checked overflow produces identical outputs across x86_64, aarch64, and riscv64 for identical inputs. TH-11 (cross-ISA determinism) is verified by the golden replay CI suite.

4. **TH-GC provides the formal bound.** Honest validators whose per-epoch perturbation stays within ε_honest are formally guaranteed (no Admitted) to never trigger H1 halt, regardless of ISA.

---

## §11.6 Formal Guarantee Summary

| Theorem | Statement | Proof file | Status |
|---|---|---|---|
| TH-3a | δ ≤ ε → halt = false | `contractivity/lyapunov_stability.v` | ✅ PROVED |
| TH-3b | halt ↔ δ > ε | `contractivity/lyapunov_stability.v` | ✅ PROVED |
| TH-3c | FinalizeEpoch → V = 0 | `contractivity/lyapunov_stability.v` | ✅ PROVED |
| TH-9 | CH_t ∈ [0,p], χ·CH_t no overflow | `cascade/cascade_health_bounded.v` | ✅ PROVED |
| **TH-GC** | **ε_honest-bounded steps → δ ≤ 3×ε_honest < ε_halt → no halt** | **`contractivity/lyapunov_grace_convergence.v`** | ✅ **PROVED** |
| TH-10 | H_cascade collision resistance | `cascade/cascade_collision_resistance.v` | Axiom (AX-3) |
| TH-SB | Blinding non-interference | `blinding/blinding_non_interference.v` | Axiom (PRF security) |
| TH-7 | Replay invariance across Tier A ISAs | golden replay CI suite | CI-verified |
| TH-11 | Cross-ISA output identity | `cascade/cascade_determinism.v` + CI | CI-verified |
