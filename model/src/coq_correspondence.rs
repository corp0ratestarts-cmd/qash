//! Coq ↔ Rust reference table.
//!
//! This module contains no executable code. It is a maintained cross-reference
//! between every public Coq definition in `proofs/` and its Rust counterpart
//! in `model/` or `crates/consensus/`. Rows that claim executable correspondence
//! link to checked artifacts and the CI jobs that exercise them.
//!
//! # How to read the table
//!
//! Each row has the form:
//!   `Coq identifier (file) ↔ Rust identifier (module)`
//!
//! "Coq" refers to definitions in the `proofs/` directory.
//! "Rust" refers to functions and types in the `model/` or `crates/` workspace.
//!
//! # Proof status legend
//! - PROVED  — fully mechanised in Coq
//! - PARTIAL — admitted lemmas remain
//! - SKETCH  — Coq file exists, significant gaps
//! - PLANNED — not yet written
//!
//! ---
//!
//! ## Model.v executable refinement surface — encoding first
//!
//! | Coq checked artifact | Rust identifier | CI job | Scope |
//! |----------------------|-----------------|--------|-------|
//! | `encode_state_header_tv0_checked` in `proofs/model/Model.v` and `proofs/model/encoding_vectors.json` | `encoding::encode_state_header` | `proofs`, `test-determinism` | canonical header bytes |
//! | `compute_leaf_index_tv0_checked` in `proofs/model/Model.v` and `proofs/model/encoding_vectors.json` | `encoding::compute_leaf_index` | `proofs`, `test-determinism` | leaf-index byte layout |
//! | `encode_validator_dynamic_tv0_checked` in `proofs/model/Model.v` and `proofs/model/encoding_vectors.json` | `encoding::encode_validator_dynamic` | `proofs`, `test-determinism` | 3×i128 validator dynamic bytes |
//! | `advance_epoch_idle4_observation_checked` / `advance_epoch_lyapunov_halt_observation_checked` and `proofs/model/transition_observations.json` | `transition::advance_epoch`, `lyapunov::evaluate` | `proofs`, `test-determinism` | Lyapunov transition observations (epoch, halt reason, V, δ, window) |
//!
//! Incremental refinement order: encoding first, Lyapunov transition observations
//! second, transaction nonce semantics third, and full state-root correspondence last.
//!
//! ---
//!
//! ## lyapunov_stability.v — TH-3a, TH-3b, TH-6
//!
//! | Coq | Rust | Notes |
//! |-----|------|-------|
//! | `state` (record) | `EpochState` (transition.rs) | Field-for-field correspondence |
//! | `input` (record) | `EpochInput` (transition.rs) | `update_count` ↔ `n_updates` |
//! | `v_validator` / `evaluate` (Model.v) | `lyapunov::evaluate` (lyapunov.rs) | Checked by `proofs/model/transition_observations.json`; CI jobs: `proofs`, `test-determinism` |
//! | `compute_delta_window` (Model.v) | `lyapunov::compute_delta_window` | Checked by `advance_epoch_*_observation_checked`; CI jobs: `proofs`, `test-determinism` |
//! | `epsilon` / `EPSILON` (Model.v) | `lyapunov::EPSILON` = 20_000 | Fixed-point, scale=1_000_000; CI job: `proofs` |
//! | `advance_epoch` / `step` (Model.v) | `transition::advance_epoch` | Lyapunov observation subset checked by `crates/consensus/tests/coq_refinement_vectors.rs`; CI job: `test-determinism` |
//! | `run` (Model.v) | repeated `transition::advance_epoch` | Halt-window setup checked by `LYAP-HALT-4-900K`; CI jobs: `proofs`, `test-determinism` |
//! | `TH-3a` (lemma) | `coq_lyapunov_transition_observations_match_advance_epoch` | δ=0 ≤ ε → no halt; artifact: `proofs/model/transition_observations.json` |
//! | `TH-3b` (lemma) | `coq_lyapunov_transition_observations_match_advance_epoch` | δ > ε → halt; artifact: `proofs/model/transition_observations.json` |
//! | `TH-6` (lemma) | `coq_model_parity` / halt-absorbing vectors | Halted state is terminal; artifact: `proofs/model/vectors.json` |
//!
//! Proof status: TH-3a PROVED, TH-3b PROVED, TH-6 PROVED. Executable subset
//! status: ENCODING FIRST (`proofs/model/encoding_vectors.json`), LYAPUNOV SECOND
//! (`proofs/model/transition_observations.json`), TRANSACTION NONCE SEMANTICS THIRD
//! (planned), FULL STATE ROOT LAST (planned).
//!
//! ---
//!
//! ## absorbing_halt.v — TH-9
//!
//! | Coq | Rust | Notes |
//! |-----|------|-------|
//! | `halt_reason` (inductive) | `HaltReason` enum (transition.rs) | None=0, variants 1..=N |
//! | `is_halted` (def) | `EpochState::is_halted` | halt_reason ≠ None |
//! | `halt_monotone` (lemma) | `tests::step_absorbs_on_halt` | Once halted, stays halted |
//! | `TH-9` (theorem) | `transition::advance_epoch` | Halt state is absorbing |
//!
//! Proof status: TH-9 PROVED (absorbing_halt.v)
//!
//! ---
//!
//! ## cascade/th10_preimage.v — TH-10
//!
//! | Coq | Rust | Notes |
//! |-----|------|-------|
//! | `cascade_step` (def) | `hash::h_domain` (hash.rs) | SHA3-256 + domain tag |
//! | `collision_resistance` (axiom) | (cryptographic assumption) | Standard ROM assumption |
//! | `TH-10` (theorem) | `hash::tests::cascade_determinism_same_input` | Determinism under SHA3 |
//!
//! Proof status: TH-10 PARTIAL (cascade axiom admitted pending formal model)
//!
//! ---
//!
//! ## cascade/th11_domain_separation.v — TH-11
//!
//! | Coq | Rust | Notes |
//! |-----|------|-------|
//! | `DomainTag` (inductive) | `hash::DomainTag` enum | Tags injective by repr(u32) |
//! | `separation_lemma` (lemma) | `hash::tests::h_domain_different_tags_produce_different_output` | Different tags → different outputs (ROM) |
//! | `TH-11` (theorem) | Composition of TH-10 + separation_lemma | Full separation |
//!
//! Proof status: TH-11 PLANNED
//!
//! ---
//!
//! ## cascade/lsh256_props.v — LSH-256 properties (planned)
//!
//! | Coq | Rust | Notes |
//! |-----|------|-------|
//! | `lsh256_collision_resistance` (axiom) | `lsh256::lsh256` (lsh256.rs) | KS X 3262 compliant |
//! | `lsh256_domain_separation` (lemma) | `lsh256::lsh256_domain` | Uses DomainTag prefix |
//!
//! Proof status: PLANNED (target: next proof sprint)
//!
//! ---
//!
//! ## Arithmetic axioms (00_execution_model.md §A4)
//!
//! | Axiom | Rust enforcement |
//! |-------|-----------------|
//! | AX-1 (no unsafe) | `#![forbid(unsafe_code)]` in qash-consensus |
//! | AX-2 (checked arithmetic) | `FixedPoint::checked_*` wraps → `Halt::absorbing_reset()` |
//! | AX-3 (no float) | no f32/f64 in Domain A (enforced by review) |
//! | AX-4 (no usize in state) | `u32`/`u64` for all wire-format fields |
//! | AX-5 (deterministic map) | `BTreeMap` mandated over `HashMap` in Domain A |
//!
//! ---
//!
//! ## Invariant numbering (for proof obligation tracking)
//!
//! | Invariant | Description | Status |
//! |-----------|-------------|--------|
//! | INV-1 | `epoch` is strictly monotone | PROVED (TH-6) |
//! | INV-2 | `halt_reason` is monotone (None → non-None) | PROVED (TH-9) |
//! | INV-3 | `slash_accum` is non-decreasing per validator | CHECKED (validate step) |
//! | INV-4 | `state_root` changes on every non-halted epoch | Implied by entropy advance |
//! | INV-5 | V_convergence ≥ 0 (FixedPoint is non-negative) | TYPE-LEVEL |
//! | INV-6 | δ_window ≤ V_convergence (min ≥ 0) | TYPE-LEVEL |
