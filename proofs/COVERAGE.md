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
| TX-0 (NoOp): V_convergence unchanged | §A8 Form A | **PROVED** | `TX0_perturbation_zero` in `contractivity/tx_perturbation_0.v` | `src/transaction.rs` | `axioms.rs::axiom_a8_form_a_tx0_zero_perturbation`; `differential.rs::diff_tx0_v_convergence_unchanged` |
| TX-1 (score decrement): V_convergence non-increasing | §A8 Form A | **PROVED** | `TX1_score_decrement_nonincreasing` in `contractivity/tx1_score_decrement.v` | `src/transaction.rs` | `differential.rs::diff_tx1_score_decrement_nonincreasing` |

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
| QASH-CASCADE-7 genesis collision resistance: requires all 7 L1 primitives (SHA3-512, BLAKE3-XOF, KangarooTwelve-XOF, SM3-double-width, Streebog-512, Kupyna-512, LSH-512) to be simultaneously broken, OR SHA3-512 (L2 binding / L7 finalization) to be broken. Trust class: ASSUMED (computational, not proved) | §4c, AX-3 | **AXIOM** | `cascade_collision_implies_sha3_collision` (axiom) + `TH10_cascade_collision_resistance` (proved wrapper) in `cascade/cascade_collision_resistance.v` — typed reduction axiom (non-vacuous); `cascade_hash_injective` proved from it | `src/cascade.rs` | `tests/cascade_kat.rs::cascade_kat_all_vectors` |
| Cascade health CH_t ∈ [0, p]; χ·CH_t no i128 overflow | §4c | **PROVED** | `ch_t_upper_bound`, `ch_term_admissible` in `cascade/cascade_health_bounded.v` | `src/crypto/cascade_coq.rs` (`CascadeHealthFactor`, `P`, `CHI`) | `cascade_coq::tests::ch_t_*`, `chi_term_*` |
| Blinding non-interference (PRF security of H_cascade_keyed) | §6 | **PROVED** (under `cascade_prf_security` axiom) | `blinding_non_interference` + `blinding_advantage_bound` + `TH_BPRF_cascade_prf` proved in `blinding/blinding_non_interference.v`; `cascade_prf_security` and `cascade_prf_quantitative_bound` are accepted axioms (typed adv_le, non-vacuous) | `src/blinding.rs` | `blinding.rs::tests::*` |
| 8-family cascade IT-MAC forgery ≤ 16/2¹²⁸ | §derive | **PROVED** (under `ghash_poly_mac_au_bound` axiom) | `it_mac_forgery_bound_at_16_blocks` (arithmetic, no axioms) + `it_mac_forgery_bound_16` + `TH_ITMAC_forgery_cap_16` proved in `cascade/it_mac_forgery_bound.v`; `ghash_poly_mac_au_bound` is accepted axiom (typed adv_le, non-vacuous) | `src/derive.rs` | `derive::tests::gf128_mul_*` |

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

## v1.1 Causal Fingerprint Properties (2-L)

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|---------|
| Fingerprint computation is deterministic: equal inputs → equal fingerprint | §2, §4 | **PROVED** | `fingerprint_deterministic` in `safety/causal_fingerprint.v` | `src/transition.rs` | `interpreter_conformance.rs::prop_p5_fingerprint_deterministic` |
| Single-step hash injectivity: H(fp,ep,r) = H(fp',ep',r') → fp=fp', ep=ep', r=r' | §2 | **PROVED** | `fp_step_injective` in `safety/causal_fingerprint.v` (from Axiom `fp_hash_injective`) | `src/transition.rs` | — |
| Fingerprint chain injectivity: equal-length chains with equal final fps had equal histories | §2, §4 | **PROVED** | `fingerprint_chain_injective` in `safety/causal_fingerprint.v` (from Axiom `fp_chain_collision_resistant`) | `src/transition.rs` | — |
| Bisimulation collapse prevention: fp-bisimilar sequences are identical | §2, §4 | **PROVED** | `bisim_collapse_prevented` in `safety/causal_fingerprint.v` | `src/transition.rs` | `interpreter_conformance.rs::prop_p4_fingerprint_changes` |
| Divergence detected: non-equal sequences cannot be fp-bisimilar | §2, §4 | **PROVED** | `divergence_detected` in `safety/causal_fingerprint.v` | `src/transition.rs` | — |

---

## v1.1 Skip-List Confluence Properties (2-L)

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|---------|
| Skip-list advance is deterministic | §5 | **PROVED** | `skiplist_advance_deterministic` in `composition/lyapunov_confluence.v` | `src/lineage.rs` | `lineage.rs::tests::*` |
| Compression is confluent: same step sequence from same header → same final header | §5 | **PROVED** | `skiplist_compression_confluent` in `composition/lyapunov_confluence.v` | `src/lineage.rs` | — |
| Canonical form is unique: run_chain is a pure function | §5 | **PROVED** | `canonical_form_unique_strong` in `composition/lyapunov_confluence.v` | `src/lineage.rs` | — |
| Deterministic replay: cross-ISA identical headers from same genesis | §1, §5 | **PROVED** | `cross_isa_replay_invariant` in `composition/lyapunov_confluence.v` | `src/lineage.rs` | `scripts/replay_test.sh` |
| Prefix consistency: append-monotone — partial replay consistent with full replay | §5 | **PROVED** | `prefix_consistent` in `composition/lyapunov_confluence.v` | `src/lineage.rs` | — |

---

## v1.1 Ordering Properties

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|---------|
| Causal sort key is deterministic: equal inputs → equal sort keys | §2, §3 | **PROVED** | `sort_key_deterministic` in `ordering/causal_ordering.v` | `src/causal_order.rs` | `causal_order.rs::tests::*` |
| `(epoch, sort_key)` lexicographic order is strict total (irrefl, trans, total) | §3 | **PROVED** | `epoch_sortkey_lt_irrefl`, `epoch_sortkey_lt_trans`, `epoch_sortkey_lt_total` in `ordering/causal_ordering.v` | `src/causal_order.rs` | — |
| Sort order is deterministic: same position list → same processing order | §3 | **PROVED** | `sort_order_deterministic` in `ordering/causal_ordering.v` | `src/causal_order.rs` | — |
| V1.0 envelope accepted before compatibility window (epoch < 100) | §3, GENESIS | **PROVED** | `version_v1_0_accepted_before_window` in `ordering/compatibility_window.v` | `src/transition.rs` | `transition::tests::version_gate_accepts_v1_0_before_window` |
| V1.0 envelope rejected at or after compatibility window (epoch ≥ 100) | §3, GENESIS | **PROVED** | `version_v1_0_rejected_after_window` in `ordering/compatibility_window.v` | `src/transition.rs` | `transition::tests::version_gate_rejects_v1_0_after_window` |
| V1.1+ always accepted regardless of epoch | §3 | **PROVED** | `version_v1_1_always_accepted` in `ordering/compatibility_window.v` | `src/transition.rs` | `transition::tests::version_gate_accepts_v1_1_after_window` |
| Window closure monotone: once past window, all future epochs reject V1.0 | §3 | **PROVED** | `window_closure_monotone`, `v1_0_rejected_all_future` in `ordering/compatibility_window.v` | `src/transition.rs` | — |

---

## Epoch Skew Validation (2-C)

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|---------|
| Epoch skew validation: envelopes with epoch < genesis_epoch or epoch > current + skew_bound are rejected; overflow on checked_add → EpochOverflow | §3, GENESIS epoch.timing.epoch_skew_bound | **CI-VERIFIED** | — (formal proof deferred to 2-I) | `crates/consensus/src/transition.rs` | `transition::tests::validate_epoch_rejects_pre_genesis`; `transition::tests::validate_epoch_rejects_too_far_future`; `transition::tests::validate_epoch_accepts_within_window`; `transition::tests::validate_epoch_overflow_on_add` |

---

## Cascade Health Tracking (2-D)

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|---------|
| Cascade health tracking: `cascade_health` increments each clean epoch (all validators D=0, C=0), resets to 0 on any gap, saturates at `CASCADE_DEPTH=8`; committed to state root; finality gate stalls (not halts) when epoch > COMPATIBILITY_WINDOW and health < CASCADE_DEPTH; Lyapunov potential gains a cascade health deficit term (weight=50_000) increasing convergence pressure during unhealthy runs | §2-D, GENESIS [cascade.health] | **CI-VERIFIED** | — | `crates/consensus/src/transition.rs`, `crates/consensus/src/lyapunov.rs` | `transition::tests::cascade_health_increments_on_clean_epochs`; `transition::tests::cascade_health_saturates_at_depth`; `transition::tests::cascade_health_resets_on_high_divergence`; `transition::tests::cascade_health_overflow_triggers_arith_overflow`; `transition::tests::finality_gate_stalls_at_epoch_101_health_7`; `transition::tests::finality_gate_passes_at_health_8`; `transition::tests::cascade_health_in_state_root_commitment`; `lyapunov::tests::lyapunov_pressure_higher_at_health_0_than_health_7`; `lyapunov::tests::lyapunov_cascade_term_zero_at_full_health` |

---

## Version Gating (2-F)

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|---------|
| Version gating: `HaltReason::IncompatibleVersion = 0x08` (H8) round-trips through encode/decode; `validate_envelope_version` rejects v1.0 envelopes after `compatibility_window=100` epochs; v1.0 accepted at or before window; v1.1+ accepted regardless of epoch; `advance_epoch` enforces the gate via `step_1_validate` | §3, GENESIS [cascade] compatibility_window | **CI-VERIFIED** | — (formal proof deferred to 2-I) | `crates/consensus/src/transition.rs` | `transition::tests::halt_reason_incompatible_version_roundtrips`; `transition::tests::validate_envelope_version_rejects_v10_after_window`; `transition::tests::validate_envelope_version_accepts_v10_at_or_before_window`; `transition::tests::validate_envelope_version_accepts_v11_after_window`; `axioms::axiom_all_halt_reasons_roundtrip` |

---

## Domain Crossing Properties (1-A)

| Property | Spec ref | Status | Coq theorem | Rust file | Test ID |
|----------|----------|--------|-------------|-----------|----------|
| CapToken schema correctness: wrap ∘ unwrap = id | §2 | **PROVED** | `cap_token_schema_correct` in `capability/cap_token_schema.v` | `crates/consensus/src/domain.rs` | — |
| CapToken wraps value (information preservation) | §2 | **PROVED** | `cap_token_wraps_value` in `capability/cap_token_schema.v` | `crates/consensus/src/domain.rs` | — |
| CapToken injectivity: equal inner values → equal tokens | §2 | **PROVED** | `cap_token_schema_injective` in `capability/cap_token_schema.v` | `crates/consensus/src/domain.rs` | — |
| Domain crossing is explicit: into_inner is the sole observation path | §2, ADR-0001 | **PROVED** | `domain_crossing_is_explicit` in `capability/cap_token_schema.v` | `crates/consensus/src/domain.rs` | — |
| All known Capability codes (0x01–0x04) pass validate_capability | §2 | **PROVED** | `capability_code_roundtrip`, `capability_code_01_valid`, `02_valid`, `03_valid` in `capability/cap_token_schema.v` | `crates/consensus/src/capability.rs` | `capability::tests::*` |
| Codes 0x00 and 0xFF are rejected | §2 | **PROVED** | `capability_code_00_invalid`, `capability_code_ff_invalid` in `capability/cap_token_schema.v` | `crates/consensus/src/capability.rs` | — |
| Capability whitelist exhaustive: codes outside whitelist are rejected | §2 | **PROVED** | `unknown_code_rejected` in `capability/cap_token_schema.v` | `crates/consensus/src/capability.rs` | — |

---

## Coverage Summary

| Status | Count |
|--------|-------|
| **PROVED** | 43 |
| **CI-VERIFIED** | 7 |
| **AXIOM** | 3 |
| **PLACEHOLDER** | 2 |
| **MISSING** | 0 |
| **Total** | 56 |

---

## v1.0 Genesis-Lock Proof-Debt Classification

Each AXIOM or PLACEHOLDER is classified by its v1.0 active claim boundary:

- **ACCEPTED** — Active v1.0 assumption; formally acknowledged; genesis-lock allowed with this axiom in place.
- **EXCLUDED** — Not part of the v1.0 active claim boundary; unclaimed functionality; no genesis-lock dependency.
- **MUST-DISCHARGE** — Must be discharged before genesis lock. *(None currently.)*

| ID | Classification | Rationale |
|----|---------------|-----------|
| GRC-7-7-v2 | **ACCEPTED** | Argon2id memory-hardness and beacon independence are accepted computational assumptions. No security proof of the GRC security reduction is claimed; the assumption is documented and bounded. |
| TH-10 (cascade collision) | **ACCEPTED** | Cascade collision resistance is an accepted cryptographic assumption (AX-3). The v1.0 state-root commitment uses SHA3-256, not the cascade, so this is a post-genesis migration concern. The SHA3-256 collision resistance assumption (also AX-3) is the active v1.0 gate. |
| Blinding PRF (H_cascade_keyed) | **EXCLUDED** | Domain B assumption; `H_cascade_keyed` is used for blinding key derivation, not Domain A state-root commitments. PRF security is not a v1.0 genesis-lock claim. |
| IT-MAC (forgery bound) | **EXCLUDED** | The IT-MAC is used in the cascade derive path, which is a Domain B / Phase 2 feature. Not an active v1.0 Domain A claim. Exclude from genesis-lock scope. |
| AX2-refinement | **ACCEPTED** | Compiler correctness axiom `AX2_rust_refinement` is accepted with 10 CI vector witnesses. The axiom is non-vacuous; its scope and limits are documented in `docs/refinement.md`. Strengthen post-v1.0 with additional vectors. |
| Cascade avalanche (TH-P1) | **EXCLUDED** | Avalanche is an empirical/statistical property, not a core security claim. Reclassified from formal proof target to statistical/KAT evidence. See note in `cascade_avalanche_property.v`. Genesis security rests on collision/preimage resistance (AX-3), not avalanche. |
| ORAM access non-interference (TH-P1) | **EXCLUDED** | Deferred to Domain B blinding spec; `blinding_params` not yet defined. Not a v1.0 genesis active claim. |
| ZK membership proof soundness (TH-P2) | **EXCLUDED** | Deferred to Plonky3 FRI-STARK integration; production ZK verification is not a v1.0 claim. |
| Blinding health Lyapunov monotonicity (P8) | **EXCLUDED** | `blinding_health` Lyapunov factor not implemented in `lyapunov.rs`. Not a v1.0 active feature. |

## Open proof obligations

The following properties are axiomatised or placeholders. Each represents a
known proof debt; see classification table above for genesis-lock status.

| ID | Property | Path to proof |
|----|----------|---------------|
| GRC-7-7-v2 | GRC-7-7-v2 genesis certificate — anti-grinding (Argon2id) and anti-precomputation (future public entropy) | **AXIOM** — anti-grinding is assumed from Argon2id memory-hardness (p=1, 512 MiB, t=3); anti-precomputation is assumed from independence of drand + NIST beacon + Bitcoin block entropy sources. `p=1` provides single-lane memory-hard sequential access, not a VDF. 7-of-7 hedge-root verification uses last-unbroken-root security model; formal proof of security reduction deferred. Implementation: `src/bin/genesis_cert.rs`; `crates/consensus/src/cascade.rs::h_cascade_l1_primitives`. |
| TH-10 | Cascade collision resistance | Post-genesis migration item for cascade-backed commitments/proofs; it is not an active v1.0 Domain A state-root assumption. Axiom `cascade_collision_implies_sha3_collision`; wrapper theorem `TH10_cascade_collision_resistance` proved from it; `cascade_hash_injective` proved. Completing the full proof requires defining `cascade_hash` concretely and applying the SHA3 injectivity chain. |
| TH-11 | H_cascade cross-ISA determinism | **Discharged** — `tests/cascade_kat.rs` pins 3 KAT vectors; `platform-determinism.yml` cross-verifies on aarch64 and riscv64gc via QEMU |
| Blinding PRF | H_cascade_keyed is a PRF | `blinding_non_interference`, `blinding_advantage_bound`, `TH_BPRF_cascade_prf` now proved in Coq. Underlying axioms (`cascade_prf_security`, `cascade_prf_quantitative_bound`) are accepted computational assumptions (typed adv_le, non-vacuous). Phase 3-A complete. |
| IT-MAC | GF(2¹²⁸) forgery bound 16/2¹²⁸ | `it_mac_forgery_bound_at_16_blocks` (pure arithmetic), `it_mac_forgery_bound_16`, `TH_ITMAC_forgery_cap_16` all proved. `ghash_poly_mac_au_bound` is an accepted typed axiom. Phase 3-B complete. |
| AX2-refinement | Coq ↔ Rust observational equivalence | Axiom `AX2_rust_refinement` in `model/RefinementStatement.v`; supported by 10 CI test vectors. Strengthen by adding more vectors to `vectors.json`/`coq_vectors.rs`, or by embedding Rust semantics in Coq (RustBelt / K-Rust, post-v1.1). |
| TH-P1 (dep) | Cascade avalanche property | `privacy/cascade_avalanche_property.v` — placeholder axiom; deferred to Domain B blinding spec revision. Proof requires SSProve/CryptHOL formalisation of the L1 random oracle model and 5-way XOR combiner. |
| TH-P1 (dep) | ORAM access non-interference | `privacy/oblivious_access_non_interference.v` — placeholder axiom; deferred to Domain B blinding spec revision and blinding_params definition. |
| TH-P2 (dep) | ZK membership proof soundness | `privacy/receipt_proof_soundness.v` — placeholder axiom; deferred to receipt spec (`06_receipts.md`) and Plonky3 FRI-STARK integration. |
| P8 | Blinding health Lyapunov monotonicity | `privacy/blinding_health_metric.v` — placeholder axioms `blinding_health_bounded`, `blinding_halt_monotone`; deferred to §P8 metric and weight definition in Domain B blinding spec. `blinding_health` Lyapunov factor is not yet implemented in `lyapunov.rs`. |
