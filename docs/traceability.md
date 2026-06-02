# QASH Spec → Code → Test → Proof Traceability

> **Normative source:** `spec/pdf/QASH_Spec_v1.0.pdf`
> **PDF-verified:** 2026-06-01
> **SHA-256:** `836985b4518df2af1a25e4fee9d7d1bb26ee9b1b2af96cf147fbd902f56a7722`
>
> All section and page citations below have been verified against the committed PDF.
> Phase 1-D (manual PDF traceability review) is complete.

## Rules

1. The **PDF quote** field must contain verbatim PDF text with section and page.
2. If the PDF does not define a requirement for a topic, the row must say
   **`PDF-SILENT`** and link an ADR that defines it.
3. No row may claim ✅ without both a code link and a test/vector link.
4. 🔶 means blocked on erratum or ADR resolution. No compliance judgment is made
   until the blocking item is resolved.
5. Mirrors may not introduce requirements not traceable to this table.

## Status key

- ✅ **Covered:** code exists, tests/vectors pass, and proof obligations compile
  if applicable.
- ⚠️ **Partial / accepted release-boundary assumption:** code exists but lacks
  tests, vectors, proofs, or has known gaps. When the Gap field reads "accepted
  release-boundary assumption", the gap is acknowledged by the project owner and
  is not a blocker for the v1.0 RC milestone. AX2 (proof-to-code extraction
  equivalence) is the most common accepted gap; it is classified as an AXIOM in
  the Coq development, not an unresolved deficiency.
- ❌ **Blocker / not finalized:** required artifact is absent or the item is
  explicitly gated on a future owner decision (e.g., Outcome A genesis-candidate).
  P0-9 (genesis hash) is the only ❌ row; it stays ❌ until Outcome A.
- 🔶 **Blocked:** compliance cannot be assessed until erratum or ADR resolution.

---

## P0 — Genesis-Lock Prerequisites

### P0-1: Lyapunov function definition and halt condition

| Field | Value |
|-------|-------|
| **PDF §** | §3.8 (§4a — Operational Lyapunov Candidate V_convergence, pp. 27–28), §3.8.3 (§4b — Safety Accumulator Φ_safety, pp. 29–30), §3.9 (§5 — Stability Criterion, pp. 30–31) |
| **PDF quote** | `V_convergence(S_t) = Σ_i [ α · D_i,t + β · C_i,t ] + χ · CH_t` (§3.8.1, p. 27) / `Φ_safety(S_t) = Σ_i [ γ · Σ_i,t ]` (§3.8.3, p. 29) / `CONDITION 1 FAIL iff δ_window > ε → absorbing halt` / `CONDITION 2 FAIL iff Φ_safety(S_t) ≥ Φ_max_safe → absorbing halt` (§3.9, p. 30–31) |
| **Code** | `crates/consensus/src/lyapunov.rs` computes `V_convergence = α·D + β·C + χ·CH` and `phi_safety = γ·Σ(slash_i)` (ADR-001/002); `PHI_MAX_SAFE = 500_000_000` pinned; `LyapunovEval.phi_halt_triggered` set when `phi ≥ PHI_MAX_SAFE`. `crates/consensus/src/transition.rs` checks `delta_window > ε` (H1) and `phi_halt_triggered` (H7) before commit. |
| **Test / Vector** | Unit tests in `lyapunov.rs`: `phi_safety_sums_across_validators` (distinguishes sum from max), `phi_halt_triggers_at_threshold` (H7 boundary). |
| **Proof** | `proofs/contractivity/lyapunov_stability.v` (TH-3), `proofs/safety/absorbing_halt.v` (TH-4/TH-5/TH-6) compiled by `.github/workflows/ci.yml` `proofs` job; zero `Admitted` beyond AX-1/AX-2. |
| **Status** | ⚠️ |
| **Gap** | ADR-001 and ADR-002 accepted; sum aggregation, H7 gate, and Coq compilation are CI-covered. Accepted release-boundary assumption: no proof-to-code extraction-equivalence evidence (AX2 is an accepted AXIOM in the Coq development; not a scheduled deliverable). |

### P0-2: State root computation

| Field | Value |
|-------|-------|
| **PDF §** | §3.4.1 (admissibility constraint 6, p. 21), §3.5 (§2 — Canonical Encoding, pp. 21–24), §3.6.2 (transition step 9, pp. 25–26) |
| **PDF quote** | `S_t.state_root = H_consensus_domain(STATE_ROOT=0x00000001, Encode_for_commitment(S_t, prior_root(t)))` (§3.4.1, p. 21) / `state_root_t = H_consensus_domain(STATE_ROOT, Encode_for_commitment(S_t, prior_root(t)))` (§3.5.4, p. 22) / transition step 9: `S'.state_root ← H_consensus_domain(STATE_ROOT=0x00000001, Encode_for_commitment(S', prior_root(t+1)))` (§3.6.2, p. 26) |
| **Code** | `crates/consensus/src/encoding.rs` contains the canonical state-root commitment preimage encoder accepted by `docs/adr/ADR-003-state-root-and-encoding.md`; v1.0 state roots use `H_consensus_domain(STATE_ROOT, Encode_for_commitment(...))`, not cascade-derived roots. |
| **Test / Vector** | `tests/vectors/vectors.v1.json` TV-0a pins the exact state-root commitment KAT (expected SHA3-256 preimage = `a92369cc…`); `crates/consensus/tests/golden_replay.rs` and `crates/consensus/tests/domain_a_audit.rs` cover valid-state roundtrip plus canonical rejection cases. |
| **Proof** | — |
| **Status** | ⚠️ |
| **Gap** | ADR-003 accepted; the PDF defines the state root formula explicitly in §3.4.1, §3.5.4, and §3.6.2. TV-0a pins the exact KAT value. Accepted release-boundary assumption: no proof-to-code extraction-equivalence evidence (AX2 is an accepted AXIOM in the Coq development; not a scheduled deliverable). |

### P0-3: Fixed-point arithmetic

| Field | Value |
|-------|-------|
| **PDF §** | §2.3.5 (§E1 Fixed-point arithmetic, pp. 11–12), §3.2.3 (fixed-point arithmetic rules, p. 19) |
| **PDF quote** | `scale: p = 1_000_000` / `width: i64 for storage, i128 for intermediate computation` / `rounding: floor toward negative infinity` / `floor_div(a: i128, b: i128) -> i128` with Euclidean semantics (§2.3.5, pp. 11–12) / `scale: p = 1_000_000 (one unit = 1_000_000 in _p)` / `intermediate width: i128` / `overflow policy: absorbing halt` (§3.2.3, p. 19) |
| **Code** | `crates/consensus/src/fixed_point.rs` defines `FixedPoint(i128)`, `SCALE = 1_000_000`, checked operations, and deterministic floor-division semantics using `i128::div_euclid()` / `i128::rem_euclid()` per PDF §2.3.5 implementation note. |
| **Test / Vector** | Module-level tests exist in `crates/consensus/src/fixed_point.rs`; `tests/vectors/vectors.v1.json` includes a fixed-point case run by `tests/vector-runner`. |
| **Proof** | — |
| **Status** | ⚠️ |
| **Gap** | PDF defines fixed-point arithmetic in §2.3.5 and §3.2.3; implementation matches. Vector values verified against PDF-defined semantics. Accepted release-boundary assumption: no formal extraction-equivalence evidence (AX2). |

### P0-4: Cross-ISA determinism

| Field | Value |
|-------|-------|
| **PDF §** | §2.8 (§E6 — ISA Support Policy, pp. 16–17), §3.10 (§6 — Replay Invariance Theorem Statement, pp. 31–32), §3.11.2 (TH-7, p. 35), §8.4 (Gate Rule TH-7, p. 73) |
| **PDF quote** | `Tier A (primary, CI-verified): x86_64 with AVX2 / aarch64 with NEON / riscv64 with V-extension (vector)` / `RT-1 (replay invariance) is tested across at minimum all Tier A platforms in CI on every push` (§2.8, pp. 16–17) / `TH-7 Replay invariance … Verification: platform-determinism.yml + test vectors` (§3.11.2, p. 35) |
| **Code** | `.github/workflows/platform-determinism.yml` and `scripts/verify_cross_isa_identity.sh` run the canonical consensus state-root test on native `x86_64` plus QEMU-backed `aarch64` and `riscv64gc` targets, then compare target roots against the native root. |
| **Test / Vector** | TV-1 (3-epoch idle root) `state_root_canonical_seq_golden` is the primary TH-7 cross-ISA anchor; `tests/vectors/vectors.v1.json`; `crates/consensus/tests/v1_1_replay.rs`; `crates/consensus/tests/v1_2_sharded_replay.rs`; `crates/consensus/tests/vector_runner.rs`. |
| **Proof** | TH-7 is a VERIFICATION CLAIM (empirical evidence, not deductive proof) per PDF §3.11.2 (p. 35): "platform-determinism.yml provides evidence; full proof requires TH-1 discharge". |
| **Status** | ⚠️ |
| **Gap** | CI gate passes on all three Tier A ISAs (PDF §2.8). Accepted release-boundary assumption: cross-ISA hosted PAL replay artifacts and corrupt-log cross-ISA coverage (scheduled for Wave 4 PR #233). |

### P0-5: Absorbing halt semantics

| Field | Value |
|-------|-------|
| **PDF §** | §2.7 (§E5 — Absorbing Halt Semantics, pp. 15–16) |
| **PDF quote** | `H1: ΔV_t > ε (Lyapunov stability violation)` / `H2: i128 overflow in Lyapunov intermediate computation` / `H3: u64 overflow of epoch counter` / `H4: Decode(bytes) returns DecodeResult::Invalid on the local state root` / `H5: round-trip failure` / `H6: halt_flag = true in any admitted S_t` (§2.7.1, p. 15) / `Signal the PAL layer via Halt::absorbing_reset(). On embedded targets: trigger hardware watchdog. On hosted targets: std::process::exit(1).` (§2.7.2, p. 16) |
| **Code** | `crates/consensus/src/transition.rs` defines halt reasons (H1–H7), sets halt state, and returns early if already halted. ADR-004 accepted: Domain A records/returns the absorbing halt state; Domain B/PAL owns zeroization, watchdog, and non-returning behavior. |
| **Test / Vector** | TV-5 (Halt trigger H1: Lyapunov violation) and TV-6 (Halt is absorbing) in `tests/vectors/vectors.v1.json` verified by `crates/consensus/tests/vector_runner.rs`; `phi_halt_triggers_at_threshold` unit test in `lyapunov.rs`. |
| **Proof** | `proofs/safety/absorbing_halt.v` (TH-4 Φ_safety monotonicity, TH-5 Φ_safety boundedness, TH-6 halt correctness). |
| **Status** | ⚠️ |
| **Gap** | ADR-004 is **accepted** (not proposed). Domain A / Domain B halt layering is defined. PDF §2.7 covers trigger conditions, halt behavior, and succession. Accepted release-boundary assumption: H7 (Φ_safety violation) is not listed in the PDF §2.7 halt codes H1–H6 — H7 is a repository extension tracked in ADR-001/002; the PDF §3.9 stability criterion implies it. This discrepancy is an accepted errata-boundary item. |

### P0-6: Leaf index concatenation

| Field | Value |
|-------|-------|
| **PDF §** | §3.4 (§1 — State Space, Merkle leaf index note, p. 20) and §3.12 Appendix A (`obfuscation.leaf_index_bytes` → §1, p. 36) |
| **PDF quote** | `The obfuscation section of GENESIS_CONSTANTS.toml defines a separate Merkle leaf index construction: validator_id(8) ‖ epoch(8) ‖ seed(32). This 48-byte epoch-relative concatenation is used exclusively for sparse Merkle tree leaf addressing — not for validator consensus identity.` (§3.4, p. 20) |
| **Code** | `crates/consensus/src/encoding.rs` implements `compute_leaf_index(validator_id: u64, epoch: u64, epoch_seed: [u8; 32]) -> [u8; 48]` with `out[0..8].copy_from_slice(&validator_id.to_le_bytes()); out[8..16].copy_from_slice(&epoch.to_le_bytes()); out[16..48].copy_from_slice(&epoch_seed)`. |
| **Test / Vector** | `tests/vectors/vectors.v1.json` includes a leaf-index vector run by `tests/vector-runner`. |
| **Proof** | `proofs/concat_injective.v` proves concatenation injectivity for this construction; compiled by `.github/workflows/ci.yml` `proofs` job, with admitted-marker rejection and `.vo` hash recording. PDF §3.10.2 references `concat_injective` as a proof dependency for TH-1. |
| **Status** | ⚠️ |
| **Gap** | Code is byte-identical to the PDF-defined construction, vector-covered, and Coq proof CI-compiled. Accepted release-boundary assumption: no proof-to-code extraction-equivalence evidence (AX2 is an accepted AXIOM in the Coq development; not a scheduled deliverable). |

### P0-7: Formal verification CI integration

| Field | Value |
|-------|-------|
| **PDF §** | §3.10.2 (proof obligations for RT-1, p. 32), §3.11.4 (Genesis lock gate, p. 35) |
| **PDF quote** | `RT-1 is not proven here. It is the proof target for: The cross-ISA CI workflow (platform-determinism.yml) / The deterministic test vector suite / The Coq contractivity proof (proofs/contractivity/lyapunov_stability.v) / The TLA+ safety invariant (proofs/safety/)` (§3.10.2, p. 32) / `GENESIS_CONSTANTS.toml must not be locked until: TH-1, TH-2, TH-3, TH-4, TH-5, TH-6, TH-8: FORMAL (Coq compiles; zero Admitted beyond AX-1/AX-2/AX-3) — TH-7: CI-verified on x86_64; aarch64 and riscv64gc cross-ISA runs must pass before final lock` (§3.11.4, p. 35) |
| **Code** | `.github/workflows/ci.yml` has a `proofs` job that installs Coq, records the Coq version, rejects active `Admitted`/`admit` markers, checks new axiom declarations against `proofs/COVERAGE.md`, compiles the active Coq proof set, hashes generated `.vo` files, and uploads `proof-coq-version.txt` plus `proof-hashes.txt` as artifacts. |
| **Test / Vector** | CI proof artifacts: `proof-coq-version.txt` and `proof-hashes.txt` uploaded as `proof-objects-${{ github.sha }}`. |
| **Proof** | Active Coq files under `proofs/` compiled explicitly by CI, including `proofs/crypto_game_framework.v`, `proofs/util/list_inj.v`, `proofs/concat_injective.v`, contractivity, safety, integration, cascade, blinding, and model files. |
| **Status** | ⚠️ |
| **Gap** | Coq compilation, admitted-marker rejection, axiom coverage checking, proof-object hashing, and artifact upload are CI-covered. The PDF §3.10.2 references "The TLA+ safety invariant (proofs/safety/)" as a proof target — this is a reference to a proof obligation, not an explicit CI command requirement; TLA+/Apalache is advisory/post-v1.0 per errata (see Wave 2 PR #230). Accepted release-boundary assumptions: AX2 proof-to-code extraction equivalence (AX2 is an accepted AXIOM in the Coq development; not a scheduled deliverable) and independent `.vo` hash reproducibility outside GitHub Actions. |

### P0-8: Pinned Rust toolchain

| Field | Value |
|-------|-------|
| **PDF §** | §2.9 (§E7 — Compilation Requirements, p. 17) |
| **PDF quote** | `builds must be reproducible across: Compiler versions (pinned via rust-toolchain.toml) / Build timestamps (suppressed via SOURCE_DATE_EPOCH=0) / Link IDs (suppressed via --build-id=none) / Incremental artifacts (disabled via CARGO_INCREMENTAL=0) / Codegen units (single via codegen-units=1 in release profile)` / `Any binary that is not reproducible under these constraints is not a conforming QASH build` (§2.9, p. 17) |
| **Code** | `rust-toolchain.toml` pins `channel = "1.95.0"`; `.github/workflows/ci.yml`, `.github/workflows/platform-determinism.yml`, and `.github/workflows/fuzz-smoke.yml` install that pinned toolchain and run `scripts/verify_rust_toolchain.sh` to print and check `rustc --version --verbose`. ADR-005 documents the revision from the PDF's draft reference of Rust 1.75.0 to the active pin 1.95.0 (Rust 1.75.0 is no longer viable with the current Cargo lockfile format). |
| **Test / Vector** | `release-attestation.yml` runs the two-stage deterministic build verification and uploads to Rekor transparency log. |
| **Proof** | — |
| **Status** | ⚠️ |
| **Gap** | Rust 1.95.0 is pinned and CI-verified; reproducible build attestation is uploaded to Rekor. Accepted release-boundary assumption: cross-ISA multi-target binary reproducibility evidence (not just state-root equivalence — full byte-identical binary) required before final genesis lock. |

### P0-9: Genesis hash

| Field | Value |
|-------|-------|
| **PDF §** | §3.5.7 (Genesis hash procedure normative, p. 24), §3.11.4 (Genesis lock gate, p. 35) |
| **PDF quote** | `genesis_hash = "SHA3-256:<64 lowercase hex digits>"` (§3.5.7, p. 24) / `genesis_hash in GENESIS_CONSTANTS.toml must be set to the SHA3-256 of the canonical spec document set (see §2 genesis hash procedure)` / `GENESIS_CONSTANTS.toml must not be locked until: TH-1, TH-2, TH-3, TH-4, TH-5, TH-6, TH-8: FORMAL … TH-7: CI-verified on all three Tier A ISAs` (§3.11.4, p. 35) |
| **Code** | `GENESIS_CONSTANTS.toml` records a recomputable pre-lock artifact-set digest and explicitly marks `genesis_status = "provisional"` with `deployment_authoritative = false`. `scripts/verify_genesis_hash.sh` recomputes the digest from `spec/genesis-artifacts.txt`. `src/bin/genesis_hash.rs` implements the SHA3-256 over the canonical doc set per §3.5.7. |
| **Test / Vector** | `.github/workflows/ci.yml` and `.github/workflows/genesis-guard.yml` run `./scripts/verify_genesis_hash.sh`. |
| **Proof** | — |
| **Status** | ❌ blocker — not finalized until Outcome A |
| **Gap** | Terminal gate. PDF §3.5.7 defines the genesis hash procedure; `spec/genesis-artifacts.txt` records the verified PDF SHA-256. The hash cannot be locked until: all P0 rows are ✅ or ⚠️ accepted release-boundary assumption, all upstream waves (PDF verification, axiom classification, proof evidence, receipt encryption, stubs, benchmarks, compliance) are complete, and owner explicitly chooses genesis-candidate in a future PR with `[genesis-change-acknowledged]`. |

---

### P1-5: Hosted PAL nondeterminism boundary

| Field | Value |
|-------|-------|
| **PDF §** | §5 (hosted/platform abstraction, provisional) |
| **PDF quote** | `PDF-SILENT`: the provisional PDF does not define the hosted crash-recovery log format or Domain B → Domain A ingress allow-list. |
| **Code** | `crates/pal/src/lib.rs` implements `hosted::Host`, canonical input records, accepted-input persistence, replay from genesis, and Domain-B-only time/network/attestation/reset helpers. |
| **Test / Vector** | `crates/pal/tests/hosted_replay.rs` replays the same persisted input log from genesis after a simulated crash/restart and checks identical state roots. |
| **Threat Model** | `docs/threat_model/nondeterminism.md` defines the Domain B → Domain A boundary and the minimal hosted runtime milestone. |
| **Proof** | — |
| **Status** | ⚠️ |
| **Gap** | Hosted PAL replay determinism has integration coverage on the native test target; cross-ISA hosted replay artifacts and corrupt-log fuzzing remain future work. |

## Phase 5 — Landed Features (PR #213)

### P5-1: Sharded Protocol Structure and EFB (5-A)

| Field | Value |
|-------|-------|
| **PDF §** | §12 (sharded protocol, provisional) |
| **PDF quote** | `PDF-SILENT`: `docs/spec/12_sharded_protocol.md` is the normative source until the PDF is committed. |
| **Code** | `crates/consensus/src/sharding.rs` — `assign_shard` computes `H_domain(ShardAssignment, epoch_seed ‖ validator_id ‖ bond_weight ‖ shard_count)` truncated mod `shard_count`; Domain A safe (no std, no unsafe, checked arithmetic). |
| **Test / Vector** | `crates/consensus/tests/shard_capture_simulation.rs` — SIM-SC-1 (uniform distribution), SIM-SC-2 (25% adversary ≤5/10 epoch captures), SIM-SC-3 (bond weight sensitivity), SIM-SC-4 (epoch seed rotation), SIM-SC-5 (40% adversary 0/100 full captures). |
| **Proof** | — (statistical simulation evidence; formal shard-security proof deferred) |
| **Status** | ✅ |
| **Gap** | Formal Coq shard-security bound proof deferred; simulation evidence gates are CI-enforced. |

---

### P5-0: Bat Immunology Tolerance Model (§11.5, TH-GC)

| Field | Value |
|-------|-------|
| **PDF §** | §11.5 (Tolerance-Based Divergence Containment) |
| **PDF quote** | `PDF-SILENT`: the bat immunology metaphor is a conceptual mapping; the formal mathematical bound is defined in `docs/spec/01_consensus.md §5` and the Coq proofs. |
| **Code** | `crates/consensus/src/lyapunov.rs` — `tolerance_margin_remaining()` returns `EPSILON_HALT - delta_window`, the remaining margin before halt. `EPSILON_HONEST = 2_000` (honest ISA variance bound); `EPSILON_HALT = 20_000` (halt threshold); safety margin = 10×. |
| **Test / Vector** | `lyapunov.rs::tests::tolerance_margin_remaining_at_zero_delta`; `lyapunov.rs::tests::tolerance_margin_remaining_at_epsilon_honest`; `golden_replay.rs::axiom_delta_window_at_epsilon_does_not_halt`. |
| **Proof** | `TH_GC_grace_no_halt`, `TH_GC_honest_steps_no_halt` — honest epochs with δ ≤ ε_honest never trigger halt. `TH_GC_tolerance_margin_positive` — 10× safety margin between ε_honest and ε_halt is formally positive. All in `contractivity/lyapunov_grace_convergence.v`. |
| **Status** | ✅ |
| **Gap** | The biological metaphor ("clonal deletion", "cross-reactive tolerance") is a documentation aid only; the mathematical bound is fully formalised and CI-enforced. |

---

### P5-2: Plonky3 FRI-STARK 2-Layer Recursion KAT (5-A)

| Field | Value |
|-------|-------|
| **PDF §** | §12 (ZK proof aggregation, provisional) |
| **PDF quote** | `PDF-SILENT`: commitment scheme and recursion profile specified in `docs/adr/` for QASH PR#93 profile. |
| **Code** | `crates/pal/src/zk/backend.rs` — `ZkBackend::prove_shard` / `ZkBackend::aggregate_shards`; `commitment_of_public_values` computes SHA3-256 over BabyBear public inputs; `FibonacciAir` test circuit. Feature-gated `plonky3,std`. |
| **Test / Vector** | `crates/pal/src/zk/backend.rs::tests::two_layer_recursion_corpus_kat_commitment` — pins SHA3-256(0x00000000 ‖ 0x01000000 ‖ 0x15000000) = `e230d00c…30a6`; `two_layer_pipeline_e2e_fibonacci` — full 4-shard layer-1 + 1 layer-2 aggregation roundtrip. |
| **Proof** | — (KAT pins the commitment scheme; full soundness proof of FRI-STARK is in the Plonky3 upstream) |
| **Status** | ✅ |
| **Gap** | Production FRI config (128 queries, full security) is too slow for unit tests; test config uses minimal parameters. Production config validation deferred to deployment profiling. |

---

## P1+ — Accepted Deferred Work Items (not v1.0 RC blockers)

All items in this table are **accepted deferred gaps** — acknowledged by the project
owner and explicitly not required for the v1.0 RC milestone. They are not unresolved
deficiencies; they are scheduled for post-RC phases. "(provisional)" in the PDF §
column means that PDF section is itself provisional in the v1.0 spec.

| ID | PDF § | Topic | Deferred because | Status |
|----|-------|-------|------------------|--------|
| P1-1 | §3.1 (pp. 5–6, provisional) | Multi-primitive cascade verification | Crypto integration phase. | Accepted deferred |
| P1-2 | §4.3 (p. 10, provisional) | Dual-path verification | Requires cascade verification. | Accepted deferred |
| P1-3 | §3.5 (pp. 8–9, provisional) | Crypto agility schedule | Requires cascade selection and vector coverage. | Accepted deferred |
| P1-4 | §5 | Hardware abstraction and deployment tiers | PAL implementation phase. | Accepted deferred |
| P1-5 | §5 | Hosted PAL nondeterminism boundary | Minimal hosted runtime implemented; cross-ISA replay artifacts deferred. | Accepted deferred |
| P1-6 | §6 | Obfuscation VM | Later subsystem phase. | Accepted deferred |
| P1-7 | §7 | Clone protocol | Later subsystem phase. | Accepted deferred |
| P1-8 | PDF-SILENT | Runtime optimization track | Scheduled by `docs/adr/ADR-006-runtime-optimization-track.md`; implementation deferred until parity and benchmark gates exist. | Accepted deferred |
| P2-1 | §9.1 (pp. 24–25, provisional) | `vm_correctness` proof | Depends on obfuscation VM. | Accepted deferred |
| P2-2 | §9.1 (pp. 24–25, provisional) | `decoy_state_identity` proof | Depends on obfuscation VM. | Accepted deferred |
