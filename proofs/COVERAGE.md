# Proof Coverage Map

Every stated protocol property is listed here with its current verification status,
the Coq theorem that proves it (if any), the Rust file that implements it, and the
test(s) that exercise it at runtime.

**Status key:**
- `PROVED` — theorem compiles under `coqc` with zero `Admitted` markers
- `CI-VERIFIED` — verified by cross-ISA CI rather than Coq (e.g., determinism replay)
- `AXIOM` — assumed; justification documented; full proof deferred
- `PLACEHOLDER` — Coq file exists but theorem body is axiomatised pending model
- `MISSING` — no proof or test exists yet

---

## Safety Properties

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|---------|
| Encoding injectivity: `Encode(S₁) = Encode(S₂) → S₁ = S₂` | §7 | **PROVED** | `TH1_encode_state_injective` in `contractivity/encode_injectivity.v` | `src/encoding.rs` | `coq_vectors.rs::coq_model_parity` |
| Encoding totality: Encode defined on all well-formed states | §7 | **PROVED** | `TH2_encode_state_total` in `contractivity/encode_injectivity.v` | `src/encoding.rs` | `golden_replay.rs::roundtrip_*` |
| Φ_safety monotonicity: slash accumulator never decreases | §4b, §5 | **PROVED** | `TH4_phi_safety_monotone` in `safety/absorbing_halt.v` | `src/transition.rs` | `axioms.rs::axiom_a1_*` |
| Φ_safety boundedness: slash accumulator ≤ Φ_max | §4b, §5 | **PROVED** | `TH5_phi_safety_bounded` in `safety/absorbing_halt.v` | `src/transition.rs` | `axioms.rs::axiom_a1_*` |
| Halt is terminal: no transition from halted state | §5 | **PROVED** | `TH6_halt_terminal`, `TH6_halt_irreversible` in `safety/absorbing_halt.v` | `src/transition.rs` | `axioms.rs::axiom_a6_halt_flag_never_clears` |
| Halted state uniqueness: same root → same state after halt | §7 | **PROVED** | `TH8_full_uniqueness` in `integration/th8_composition.v` | `src/transition.rs`, `src/encoding.rs` | `golden_replay.rs::state_root_is_deterministic` |

---

## Liveness / Convergence Properties

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|---------|
| No halt when δ_window ≤ ε | §4a | **PROVED** | `TH3a_no_halt_within_epsilon` in `contractivity/lyapunov_stability.v` | `src/lyapunov.rs` | `golden_replay.rs::within_epsilon_does_not_halt` |
| Halt iff δ_window > ε (equivalence) | §4a, §4b | **PROVED** | `TH3b_halt_iff_delta_exceeds_epsilon` in `contractivity/lyapunov_stability.v` | `src/lyapunov.rs` | `golden_replay.rs::axiom_delta_window_*` |
| FinalizeEpoch drives V_convergence → 0 | §4b, §4c | **PROVED** | `TH3c_finalize_zero` in `contractivity/lyapunov_stability.v` | `src/lyapunov.rs` | `golden_replay.rs::prop_lyapunov_nonneg` |
| Grace period: honest epochs never trigger halt (ε_honest = 2k, ε = 20k) | §11.5 | **PROVED** | `TH_GC_grace_no_halt`, `TH_GC_honest_steps_no_halt` in `contractivity/lyapunov_grace_convergence.v` | `src/lyapunov.rs` | `golden_replay.rs::axiom_delta_window_at_epsilon_does_not_halt` |
| Tolerance margin > 0 (10× safety margin between ε_honest and ε_halt) | §11.5 | **PROVED** | `TH_GC_tolerance_margin_positive` in `contractivity/lyapunov_grace_convergence.v` | `src/lyapunov.rs` | — |

---

## Transaction Properties

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|---------|
| TX-0 (NoOp): V_convergence unchanged | §A8 Form A | **PROVED** | `TX0_perturbation_zero` in `contractivity/tx_perturbation_0.v` | `src/transaction.rs` | `axioms.rs::axiom_a8_form_a_tx0_zero_perturbation` |
| TX-1 (score decrement): V_convergence non-increasing | §A8 Form A | **PROVED** | `TX1_score_decrement_nonincreasing` in `contractivity/tx1_score_decrement.v` | `src/transaction.rs` | `golden_replay.rs::full_epoch_with_tx0` |

---

## Determinism Properties

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|---------|
| Cross-ISA replay invariance (x86_64, aarch64, riscv64gc) | §1 | **CI-VERIFIED** | TH-7 — enforced by CI golden replay (no Coq model needed) | `src/transition.rs`, `src/encoding.rs` | `tests/replay_corpus.rs`, `tests/golden_replay.rs::state_root_canonical_seq_golden` |
| H_cascade bitwise-identical across Tier A ISAs | §4c | **CI-VERIFIED** | — enforced by cascade KAT + platform-determinism cross-ISA CI | `src/cascade.rs` | `tests/cascade_kat.rs::cascade_kat_all_vectors`, `platform-determinism.yml` |

---

## Cryptographic Properties

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|---------|
| SHA3-256 collision resistance | §7, AX-3 | **AXIOM** | `AX3_sha3_assumed_injective` in `contractivity/encode_injectivity.v` | `src/hash.rs` | `hash.rs::tests::sha3_256_known_vector` (KAT) |
| Cascade collision resistance (reduction to L1 primitive) | §4c | **PLACEHOLDER** | `cascade/cascade_collision_resistance.v` — axiomatised pending hash model | `src/cascade.rs` | — |
| Cascade health CH_t ∈ [0, p]; χ·CH_t no i128 overflow | §4c | **PROVED** | `ch_t_upper_bound`, `ch_term_admissible` in `cascade/cascade_health_bounded.v` | `src/cascade.rs` | — |
| Blinding non-interference (PRF security of H_cascade_keyed) | §6 | **AXIOM** | `cascade_prf_security` in `blinding/blinding_non_interference.v` | `src/blinding.rs` | `blinding.rs::tests::*` |
| 8-family cascade IT-MAC forgery ≤ 16/2¹²⁸ | §derive | **PLACEHOLDER** | `cascade/it_mac_forgery_bound.v` discharges arithmetic cap; AU security game reduction still axiomatised | `src/derive.rs` | `derive::tests::gf128_mul_*` |

---

## Encoding / Serialization Properties

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|---------|
| Encode/decode roundtrip | §7 | **PROVED** (via TH-1) | `TH1_encode_state_injective` | `src/encoding.rs` | `golden_replay.rs::prop_encode_decode_roundtrip` |
| State root chains via prior root (epoch linkage) | §7 | **CI-VERIFIED** | — | `src/transition.rs` | `golden_replay.rs::state_root_chains_via_prior_root` |
| State root changes each epoch | §7 | **CI-VERIFIED** | — | `src/transition.rs` | `golden_replay.rs::state_root_changes_each_epoch` |

---

## Coverage Summary

| Status | Count |
|--------|-------|
| **PROVED** | 14 |
| **CI-VERIFIED** | 5 |
| **AXIOM** | 3 |
| **PLACEHOLDER** | 2 |
| **MISSING** | 0 |
| **Total** | 24 |

---

## Open proof obligations

The following properties are axiomatised or placeholders. Each represents a
known proof debt that should be discharged before mainnet.

| ID | Property | Path to proof |
|----|----------|---------------|
| TH-10 | Cascade collision resistance | Requires formalising hash function in Coq (Whirlpool/GHASH model); consider EasyCrypt or CryptHOL |
| TH-11 | H_cascade cross-ISA determinism | **Discharged** — `tests/cascade_kat.rs` pins 3 KAT vectors; `platform-determinism.yml` cross-verifies on aarch64 and riscv64gc via QEMU |
| Blinding PRF | H_cascade_keyed is a PRF | Formal proof in CryptHOL or SSProve; current Coq axiom is a sound placeholder |
| IT-MAC | GF(2¹²⁸) forgery bound 16/2¹²⁸ | **Partially discharged** in `cascade/it_mac_forgery_bound.v` (numerical cap proved); AU game proof still open |
