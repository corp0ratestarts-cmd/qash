# QASH Protocol Migration: v1.0 → v1.1
## `docs/spec/09_migration_v1.0_to_v1.1.md`

> **Status:** Deferred migration spec. Genesis is locked at **v1.0**.
> This document specifies the future v1.1 upgrade path; it takes effect only
> when `GENESIS_CONSTANTS.toml` version is advanced to `1.1.0` and the
> cascade-health blinding activation is wired into `transition.rs`
> (currently `cascade.rs` and `blinding.rs` are compiled and tested but not
> connected to the epoch transition path — see `lib.rs` comments).
> The v1.1 cascade/blinding **proof** files (`cascade_health_bounded.v`,
> `cascade_determinism.v`, `cascade_collision_resistance.v`,
> `blinding_non_interference.v`) are compiled in CI and cover theorems for
> that future activation. The v1.1 **runtime** weight changes and CH term
> are not active.

---

## Summary of Changes

| Area | v1.0 | v1.1 |
|------|------|------|
| `GENESIS_CONSTANTS.toml` version | `1.0.0` | `1.1.0` |
| Lyapunov weight D (α) | 400_000 | 350_000 |
| Lyapunov weight C (β) | 350_000 | 300_000 |
| Lyapunov weight Σ (γ) | 250_000 | 200_000 |
| Cascade health weight CH (χ) | — | 150_000 |
| Obfuscation method | `concatenation_injective` | `cascade_derived_injective` |
| Hash cascade | Serial 3-primitive chain | Depth-7 astronomical cascade (DD-7) |
| Clone protocol version | v1.1 | v1.2 |
| Clone chunk verification | — | `cascade_bound` |
| KEM | — | ML-KEM-768 |
| PQC rotation schedule | (absent from TOML) | Pre-baked in `[crypto.cascade.rotation_schedule]` |
| New theorems | TH-1 through TH-8 | + TH-9, TH-10, TH-11 |

---

## Compatibility Window

`[migration.compatibility]` in `GENESIS_CONSTANTS.toml` defines the window
during which v1.0 validators are accepted:

```toml
accept_v1_0_validators = true
migration_window_epochs = 100
v1_0_validator_cascade_mode = "parallel_only"
post_migration_enforcement = "cascade_required"
state_conversion_proof_required = true
```

### Window semantics

- **Epochs 0–99**: v1.0 validators are admitted with `cascade_mode = parallel_only`.
  They compute cascade outputs but are not required to submit cascade inclusion proofs.
  Their cascade health contribution (`CH_t`) is counted as zero.
- **Epoch 100+**: `cascade_required`. All validators must submit valid cascade proofs.
  Validators that do not are treated as cascade-failing and contribute to `CH_t`.

### State conversion

Validators converting from v1.0 to v1.1 must:
1. Recompute `V_convergence` using new weight table (α, β, γ, χ from v1.1)
2. Compute `H_cascade` for all pending obfuscation leaves using the depth-7 construction
3. Submit `state_conversion_proof` — a sparse-Merkle proof that all existing leaf hashes
   have been recomputed under the new cascade

`state_conversion_proof_required = true` means validators that skip step 3
are treated as inadmissible from epoch 100 onwards.

---

## Weight Change Impact on Φ_max_safe

v1.0 Φ_max = 1024 × 250_000 × (2^63 − 1) ≈ 2.36 × 10^27
v1.1 Φ_max = 1024 × 200_000 × (2^63 − 1) ≈ 1.89 × 10^27

The reduced γ means the halt threshold is reached at a lower total slash
accumulation. Validators with high slash accumulators that were safe under v1.0
may be closer to the halt threshold under v1.1. The migration window provides
100 epochs for operators to assess this.

---

## Cascade Health (CH) — New Term in V_convergence

v1.0 had no cascade health term. v1.1 adds:

```
χ · CH_t   where χ = 150_000, CH_t ∈ [0, p = 1_000_000]
```

In a healthy epoch (all cascade proofs pass), `CH_t = 0` and the term
contributes nothing. A fully degraded epoch (all proofs fail) adds
`150_000 × 1_000_000 = 1.5 × 10^11` to `V_convergence`.

During the migration window (`epochs 0–99`), v1.0 validators contribute
`CH_t = 0` for their cascade proofs (as if all their proofs pass). This
prevents the new term from unfairly triggering halt during conversion.

---

## Clone Protocol v1.2 Changes

`chunk_verification_mode = "cascade_bound"` means each clone chunk now
carries a cascade inclusion proof (format defined in `07_hash_cascade.md`).
During the migration window, v1.0 chunks without proofs are accepted but
logged. After epoch 100, proofless chunks are rejected.

---

## Formal Obligations

| Obligation | Status |
|-----------|--------|
| TH-9 (CH boundedness) | ✅ PROVED — `proofs/cascade/cascade_health_bounded.v` (no `Admitted`; CI-gated) |
| TH-10 (cascade collision resistance) | ✅ AXIOM — intentionally reduces to AX-3 (SHA3-256 collision resistance); same trust class as AX-3 in `proofs/STATUS.md`. The reduction argument is: if `H_cascade` is not collision-resistant then at least one L1 primitive is breakable, which contradicts AX-3. |
| TH-11 (cascade cross-ISA determinism) | ✅ CI-VERIFIED — cross-ISA test vectors validated on all Tier A ISAs; same delegation class as TH-7. Formal Coq proof would require axiomatizing ISA semantics (AX-1); CI verification is the accepted alternative. |
| Weight-adjusted TH-3 proof | ✅ N/A for v1.0 — `proofs/contractivity/lyapunov_stability.v` uses **v1.0** weight constants (D=400k, C=350k, S=250k, no CH term). Proof is weight-agnostic; all three theorems hold for any positive weights. |
| Weight-adjusted TH-5 proof | ✅ N/A for v1.0 — `proofs/safety/absorbing_halt.v` uses v1.0 gamma=250_000. Proof structure is weight-agnostic; Phi_max = N_max × 250_000 × INT_MAX = 1024 × 250,000 × (2⁶³−1) ≈ 2.36 × 10²⁷. |

### Genesis Lock Gate — Current Status (v1.0)

**All obligations discharged at v1.0:**
- TH-9 PROVED, TH-10 AX-3 reduction, TH-11 CI-verified
- Proofs aligned to v1.0 weights (D=400k, C=350k, S=250k); no CH term
- TH-7 cross-ISA: x86_64, aarch64, and riscv64gc all produce identical state roots
  (verified via QEMU user-static; `.cargo/config.toml` configures runners; CI workflow
  at `.github/workflows/platform-determinism.yml` installs required packages)

**v1.1 migration gate (deferred):** activates when `WEIGHT_BH > 0` and
`GENESIS_CONSTANTS.toml` version is advanced to `1.1.0`. At that point,
the weight-adjusted proofs described in this table must be re-verified.

---

*End of `docs/spec/09_migration_v1.0_to_v1.1.md`*
