# Proof Coverage Map

Every stated protocol property is listed here with its current verification status,
the Coq theorem that proves it (if any), the Rust file that implements it, and the
test(s) that exercise it at runtime.

**Status key:**
- `PROVED` — theorem compiles under `coqc` with zero `Admitted` markers; active proof files are compiled by CI
- `CI-VERIFIED` — verified by non-Coq CI rather than a Coq theorem (e.g., determinism replay)
- `AXIOM` — assumed; justification documented; full proof deferred
- `PLACEHOLDER` — Coq file exists but theorem body is axiomatised pending model
- `MISSING` — no proof or test exists yet

---

## Safety Properties

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|----------|
| Encoding injectivity: `Encode(S₁) = Encode(S₂) → S₁ = S₂` | §7 | **PROVED** | `TH1_encode_state_injective` in `contractivity/encode_injectivity.v` | `src/encoding.rs` | `coq_vectors.rs::coq_model_parity` |
| Encoding totality: Encode defined on all well-formed states | §7 | **PROVED** | `TH2_encode_state_total` in `contractivity/encode_injectivity.v` | `src/encoding.rs` | `golden_replay.rs::roundtrip_*` |
| Φ_safety monotonicity: slash accumulator never decreases | §4b, §5 | **PROVED** | `TH4_phi_safety_monotone` in `safety/absorbing_halt.v` | `src/transition.rs` | `axioms.rs::axiom_a1_*` |
| Φ_safety sum aggregation and H7 threshold: `Φ_safety = W_S·Σ slash_i`; `Φ_safety ≥ PHI_MAX_SAFE` halts | §4b, §5, ADR-001/002 | **PROVED** | `TH5_phi_safety_bounded` in `safety/absorbing_halt.v` | `src/lyapunov.rs`, `src/transition.rs` | `lyapunov.rs::phi_safety_sums_across_validators`; `lyapunov.rs::phi_halt_triggers_at_threshold`; `transition.rs::evaluate_projected_phi_safety_sums_across_validators`; `transition.rs::phi_safety_halts_at_threshold_before_commit` |
| Halt is terminal: no transition from halted state | §5 | **PROVED** | `TH6_halt_terminal`, `TH6_halt_irreversible` in `safety/absorbing_halt.v` | `src/transition.rs` | `axioms.rs::axiom_a6_halt_flag_never_clears` |
| Halted state uniqueness: same root → same state after halt | §7 | **PROVED** | `TH8_full_uniqueness` in `integration/th8_composition.v` | `src/transition.rs`, `src/encoding.rs` | `golden_replay.rs::state_root_is_deterministic` |

---

## Liveness / Convergence Properties

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|----------|
| No halt when δ_window ≤ ε | §4a | **PROVED** | `TH3a_no_halt_within_epsilon` in `contractivity/lyapunov_stability.v` | `src/lyapunov.rs` | `golden_replay.rs::within_epsilon_does_not_halt` |
| Halt iff δ_window > ε (equivalence) | §4a, §4b | **PROVED** | `TH3b_halt_iff_delta_exceeds_epsilon` in `contractivity/lyapunov_stability.v` | `src/lyapunov.rs` | `golden_replay.rs::axiom_delta_window_*` |
| FinalizeEpoch drives V_convergence → 0 | §4b, §4c | **PROVED** | `TH3c_finalize_zero` in `contractivity/lyapunov_stability.v` | `src/lyapunov.rs` | `golden_replay.rs::prop_lyapunov_nonneg` |
| Grace period: honest epochs never trigger halt (ε_honest = 2k, ε = 20k) | §11.5 | **PROVED** | `TH_GC_grace_no_halt`, `TH_GC_honest_steps_no_halt` in `contractivity/lyapunov_grace_convergence.v` | `src/lyapunov.rs` | `golden_replay.rs::axiom_delta_window_at_epsilon_does_not_halt` |
| Tolerance margin > 0 (10× safety margin between ε_honest and ε_halt) | §11.5 | **PROVED** | `TH_GC_tolerance_margin_positive` in `contractivity/lyapunov_grace_convergence.v` | `src/lyapunov.rs` | — |

---

## Transaction Properties

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|----------|
| TX-0 (NoOp): V_convergence unchanged | §A8 Form A | **PROVED** | `TX0_perturbation_zero` in `contractivity/tx_perturbation_0.v` | `src/transaction.rs` | `axioms.rs::axiom_a8_form_a_tx0_zero_perturbation` |
| TX-1 (score decrement): V_convergence non-increasing | §A8 Form A | **PROVED** | `TX1_score_decrement_nonincreasing` in `contractivity/tx1_score_decrement.v` | `src/transaction.rs` | `golden_replay.rs::full_epoch_with_tx0` |

---

## Determinism Properties

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|----------|
| Cross-ISA replay invariance (x86_64, aarch64, riscv64gc) | §1 | **CI-VERIFIED** | TH-7 — enforced by CI golden replay (no Coq model needed) | `src/transition.rs`, `src/encoding.rs` | `tests/replay_corpus.rs`, `tests/golden_replay.rs::state_root_canonical_seq_golden` |
| H_cascade bitwise-identical across Tier A ISAs | §4c | **CI-VERIFIED** | — enforced by cascade KAT + platform-determinism cross-ISA CI | `src/cascade.rs` | `tests/cascade_kat.rs::cascade_kat_all_vectors`, `platform-determinism.yml` |

---

## Cryptographic Properties

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|----------|
| SHA3-256 collision resistance for v1.0 state-root commitments | §7, AX-3 | **AXIOM** | `AX3_sha3_assumed_injective` in `contractivity/encode_injectivity.v`; `sha3_256_collision_resistant` in `cascade/cascade_collision_resistance.v` | `src/hash.rs`, `src/transition.rs` | `hash.rs::tests::sha3_256_known_vector` (KAT); `vector_runner_all` (`state_root_commitment_genesis_epoch1`) |
| Cascade collision resistance (reduction to L1 primitive; post-genesis migration item, not active for v1.0 Domain A state roots) | §4c | **PLACEHOLDER** | `cascade_collision_implies_sha3_collision` (axiom) + `TH10_cascade_collision_resistance` (proved wrapper) in `cascade/cascade_collision_resistance.v` — typed reduction axiom (non-vacuous); `cascade_hash_injective` proved from it | `src/cascade.rs` | — |
| Cascade health CH_t ∈ [0, p]; χ·CH_t no i128 overflow | §4c | **PROVED** | `ch_t_upper_bound`, `ch_term_admissible` in `cascade/cascade_health_bounded.v` | `src/cascade.rs` | — |
| Blinding non-interference (PRF security of H_cascade_keyed) | §6 | **AXIOM** | `cascade_prf_security` (qualitative) + `cascade_prf_quantitative_bound` (typed adv_le bound) in `blinding/blinding_non_interference.v` | `src/blinding.rs` | `blinding.rs::tests::*` |
| 8-family cascade IT-MAC forgery ≤ 16/2¹²⁸ | §derive | **PLACEHOLDER** | `cascade/it_mac_forgery_bound.v`: arithmetic cap proved; `ghash_poly_mac_au_bound` typed axiom (adv_le, non-vacuous); `it_mac_forgery_bound_16` proved | `src/derive.rs` | `derive::tests::gf128_mul_*` |

---

## Encoding / Serialization Properties

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|----------|
| Encode/decode roundtrip | §7 | **PROVED** (via TH-1) | `TH1_encode_state_injective` | `src/encoding.rs` | `golden_replay.rs::prop_encode_decode_roundtrip` |
| State root chains via prior root (epoch linkage) | §7 | **CI-VERIFIED** | — | `src/transition.rs` | `golden_replay.rs::state_root_chains_via_prior_root` |
| State root changes each epoch | §7 | **CI-VERIFIED** | — | `src/transition.rs` | `golden_replay.rs::state_root_changes_each_epoch` |

---

## Refinement Properties

Formal correspondence between the Coq executable model (`proofs/model/Model.v`) and the
Rust implementation (`crates/consensus/src/transition.rs`). See `docs/refinement.md` for
the full three-layer correspondence chain and extraction pipeline.

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|----------|
| RT-1: Successful step advances epoch and clears halt flag | §4a, §5 | **PROVED** | `RT1_successful_step` in `model/RefinementStatement.v` | `src/transition.rs` | `coq_vectors.rs::coq_model_parity` (TV-1, TV-2, TV-3, TV-9) |
| RT-2: Over-epsilon step sets halt flag | §4a, §5 | **PROVED** | `RT2_halt_step` in `model/RefinementStatement.v` | `src/transition.rs` | `coq_vectors.rs::coq_model_parity` (TV-4) |
| RT-3: Halted step preserves epoch (absorbing) | §5 | **PROVED** | `RT3_halt_absorbing_epoch` in `model/RefinementStatement.v` | `src/transition.rs` | `coq_vectors.rs::coq_model_parity` (TV-5) |
| RT-4: Halted step preserves halt flag (absorbing) | §5 | **PROVED** | `RT4_halt_absorbing_flag` in `model/RefinementStatement.v` | `src/transition.rs` | `coq_vectors.rs::coq_model_parity` (TV-5) |
| Coq ↔ Rust observational equivalence (AX-2 refinement) | §1 | **AXIOM** | `AX2_rust_refinement` in `model/RefinementStatement.v` | `src/transition.rs` | `coq_vectors.rs::coq_model_parity` (10 vectors); `release-attestation.yml` |

---

## Coverage Summary

| Status | Count |
|--------|-------|
| **PROVED** | 18 |
| **CI-VERIFIED** | 4 |
| **AXIOM** | 3 |
| **PLACEHOLDER** | 2 |
| **MISSING** | 0 |
| **Total** | 27 |

---

## Open proof obligations

The following properties are axiomatised or placeholders. Each represents a
known proof debt that should be discharged before mainnet.

| ID | Property | Path to proof |
|----|----------|---------------|
| TH-10 | Cascade collision resistance | Post-genesis migration item for cascade-backed commitments/proofs; it is not an active v1.0 Domain A state-root assumption. Axiom `cascade_collision_implies_sha3_collision`; wrapper theorem `TH10_cascade_collision_resistance` proved from it; `cascade_hash_injective` proved. Completing the full proof requires defining `cascade_hash` concretely and applying the SHA3 injectivity chain. |
| TH-11 | H_cascade cross-ISA determinism | **Discharged** — `tests/cascade_kat.rs` pins 3 KAT vectors; `platform-determinism.yml` cross-verifies on aarch64 and riscv64gc via QEMU |
| Blinding PRF | H_cascade_keyed is a PRF | Qualitative (`cascade_prf_security`) and quantitative (`cascade_prf_quantitative_bound` with `adv_le`) axioms in place. Full proof in SSProve; current axioms non-vacuous. |
| IT-MAC | GF(2¹²⁸) forgery bound 16/2¹²⁸ | Arithmetic cap proved; `ghash_poly_mac_au_bound` typed (adv_le), `it_mac_forgery_bound_16` proved. AU game proof still open (SSProve/CryptHOL target). |
| AX2-refinement | Coq ↔ Rust observational equivalence | Axiom `AX2_rust_refinement` in `model/RefinementStatement.v`; supported by 10 CI test vectors. Strengthen by adding more vectors to `vectors.json`/`coq_vectors.rs`, or by embedding Rust semantics in Coq (RustBelt / K-Rust, post-v1.1). |
