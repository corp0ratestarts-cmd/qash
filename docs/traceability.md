# QASH Spec → Code → Test → Proof Traceability

> **Normative source:** `spec/pdf/QASH_Spec_v1.0.pdf` (QASH v1.0).
> The PDF must be checked into `spec/pdf/` before genesis lock. Until then,
> section and page references below are provisional and must be verified against
> the committed PDF.

## Rules

1. The **PDF quote** field must contain verbatim PDF text with section and page.
2. If the PDF does not define a requirement for a topic, the row must say
   **`PDF-SILENT`** and link an ADR that defines it.
3. No row may claim ✅ without both a code link and a test/vector link.
4. 🔶 means blocked on erratum or ADR resolution. No compliance judgment is made
   until the blocking item is resolved.
5. Mirrors may not introduce requirements not traceable to this table.

## Genesis-lock release gate handoff

Genesis-lock proof readiness in `proofs/STATUS.md` is bridged to release readiness via
`docs/release/genesis_lock_gates.md`, which defines owners, objective pass criteria,
and closure evidence for each non-proof blocker.

## Status key

- ✅ **Covered:** code exists, tests/vectors pass, and proof obligations compile
  if applicable.
- ⚠️ **Partial:** code exists but lacks tests, vectors, proofs, or has known gaps.
- ❌ **Missing:** no implementation or required artifact exists.
- 🔶 **Blocked:** compliance cannot be assessed until erratum or ADR resolution.

---

## P0 — Genesis-Lock Prerequisites

### P0-1: Lyapunov function definition and halt condition

| Field | Value |
|-------|-------|
| **PDF §** | §4.1 (pp. 9–10, provisional) |
| **PDF quote** | `pub struct LyapunovState { divergence: FixedPoint, conflict: FixedPoint, signature_health: FixedPoint }` / `L = W_D·D + W_C·C + W_S·Σ` / `∀t: L(t+1) - L(t) ≤ ε_threshold` / `If violated → absorbing halt` |
| **Code** | `crates/consensus/src/lyapunov.rs` computes `V_convergence = α·D + β·C` and `phi_safety = γ·Σ(slash_i)` (ADR-001/002); `PHI_MAX_SAFE = 500_000_000` pinned; `LyapunovEval.phi_halt_triggered` set when `phi ≥ PHI_MAX_SAFE`. `crates/consensus/src/transition.rs` checks `delta_window > ε` (H1) and `phi_halt_triggered` (H7) before commit. |
| **Test / Vector** | Unit tests in `lyapunov.rs`: `phi_safety_sums_across_validators` (distinguishes sum from max), `phi_halt_triggers_at_threshold` (H7 boundary). |
| **Proof** | `proofs/contractivity/lyapunov_stability.v` is compiled by the `.github/workflows/ci.yml` `proofs` job, after CI rejects active `Admitted`/`admit` markers and checks axiom coverage. |
| **Status** | ⚠️ |
| **Gap** | ADR-001 and ADR-002 accepted; sum aggregation, H7 gate, and Coq compilation are CI-covered. Remaining gaps: no proof-to-code refinement from the Coq model to Rust and no extraction-equivalence evidence. |

### P0-2: State root computation

| Field | Value |
|-------|-------|
| **PDF §** | §4.2 (p. 10, provisional) |
| **PDF quote** | `let new_state = compute_state_root(state, crypto_suite);` |
| **Code** | `crates/consensus/src/encoding.rs` contains `state_root_header_only()`, which hashes a 52-byte header. |
| **Test / Vector** | — |
| **Proof** | — |
| **Status** | 🔶 |
| **Blocking** | `PDF-SILENT`: the PDF calls `compute_state_root` but does not define the state byte layout or commitment structure. `docs/adr/ADR-003-state-root-encoding.md` must define canonical encoding and state-root input bytes before compliance can be assessed. |

### P0-3: Fixed-point arithmetic

| Field | Value |
|-------|-------|
| **PDF §** | §2.4 (pp. 4–5, provisional) |
| **PDF quote** | `pub struct FixedPoint { value: i128 }` / `const SCALE: i128 = 1_000_000` / `self.value.checked_mul(other.value).ok_or(OverflowError)?` / `product.checked_div(SCALE).ok_or(OverflowError)?` |
| **Code** | `crates/consensus/src/fixed_point.rs` defines `FixedPoint(i128)`, `SCALE = 1_000_000`, checked operations, and deterministic division semantics. |
| **Test / Vector** | Module-level tests exist in `crates/consensus/src/fixed_point.rs`; `tests/vectors/vectors.v1.json` includes a fixed-point case run by `tests/vector-runner`. |
| **Proof** | — |
| **Status** | ⚠️ |
| **Gap** | Arithmetic is now represented in the vector scaffold, but it remains code-derived until the PDF is committed and the vector values are independently verified. |

### P0-4: Cross-ISA determinism

| Field | Value |
|-------|-------|
| **PDF §** | §2.5 (p. 5, provisional) and §8.4 (pp. 23–24, provisional) |
| **PDF quote** | `all validators produce bitwise-identical outputs regardless of hardware platform` / `./scripts/verify_cross_isa_identity.sh ${{ matrix.target }}` |
| **Code** | `.github/workflows/platform-determinism.yml` and `scripts/verify_cross_isa_identity.sh` run the canonical consensus state-root test on native `x86_64` plus QEMU-backed `aarch64` and `riscv64gc` targets, then compare target roots against the native root. |
| **Test / Vector** | `state_root_canonical_seq_print`; `tests/vectors/vectors.v1.json`; `crates/consensus/tests/v1_1_replay.rs`; `crates/consensus/tests/v1_2_sharded_replay.rs`; `crates/consensus/tests/vector_runner.rs`. |
| **Proof** | — |
| **Status** | ⚠️ |
| **Gap** | CI verifies the authorized ISA roots and replay gates. Remaining gaps: local non-native QEMU execution requires the apt packages installed by `scripts/install_test_dependencies.sh`, and the state-root vector remains code-derived until ADR-003 defines full state encoding and the normative PDF is committed. |

### P0-5: Absorbing halt semantics

| Field | Value |
|-------|-------|
| **PDF §** | §2.3 (pp. 3–4, provisional) |
| **PDF quote** | `pub fn trigger_absorbing_halt(reason: HaltReason) -> ! { zeroize_critical_memory(); #[cfg(feature = "itron")] unsafe { itron_disable_scheduler() }; #[cfg(has_watchdog)] unsafe { trigger_wdt_reset() }; loop { core::hint::spin_loop() } }` |
| **Code** | `crates/consensus/src/transition.rs` defines halt reasons, sets halt state, and returns early if already halted. |
| **Test / Vector** | — |
| **Proof** | — |
| **Status** | ⚠️ |
| **Blocking** | `docs/adr/ADR-004-absorbing-halt-layering.md` must define how deterministic Domain A halt behavior composes with PAL zeroize/watchdog behavior to satisfy the PDF's diverging halt contract. |

### P0-6: Leaf index concatenation

| Field | Value |
|-------|-------|
| **PDF §** | §3.2 (pp. 6–7, provisional) |
| **PDF quote** | `pub fn compute_leaf_index(validator_id: u64, epoch: u64, epoch_seed: [u8; 32]) -> [u8; 48]` / `out[0..8].copy_from_slice(&validator_id.to_le_bytes()); out[8..16].copy_from_slice(&epoch.to_le_bytes()); out[16..48].copy_from_slice(&epoch_seed);` |
| **Code** | `crates/consensus/src/encoding.rs` implements `compute_leaf_index` with the same signature and byte layout. |
| **Test / Vector** | `tests/vectors/vectors.v1.json` includes a leaf-index vector run by `tests/vector-runner`. |
| **Proof** | PDF §9.1 names `concat_injective`; `proofs/concat_injective.v` is compiled by the `.github/workflows/ci.yml` `proofs` job, with admitted-marker rejection and `.vo` hash recording. |
| **Status** | ⚠️ |
| **Gap** | Code appears byte-identical to the PDF pseudocode, has vector coverage, and the supporting Coq file is CI-compiled. Remaining gaps: traceability still needs a precise mapping from the PDF theorem name to the compiled theorem(s), and no proof-to-code refinement or extraction-equivalence evidence exists. |

### P0-7: Formal verification CI integration

| Field | Value |
|-------|-------|
| **PDF §** | §9.3 (p. 25, provisional) |
| **PDF quote** | `cd proofs && make all` / `apalache-mc check --length=100 tla/QASHConsensus.tla` / `test -f proofs/concat_injective.vo` / `test -f proofs/vm_correctness.vo` |
| **Code** | `.github/workflows/ci.yml` has a `proofs` job that installs Coq, records the Coq version, rejects active `Admitted`/`admit` markers, checks new axiom declarations against `proofs/COVERAGE.md`, compiles the active Coq proof set, hashes generated `.vo` files, and uploads `proof-coq-version.txt` plus `proof-hashes.txt` as artifacts. |
| **Test / Vector** | CI proof artifacts: `proof-coq-version.txt` and `proof-hashes.txt` uploaded as `proof-objects-${{ github.sha }}`. |
| **Proof** | Active Coq files under `proofs/` are compiled explicitly by `.github/workflows/ci.yml`, including `proofs/crypto_game_framework.v`, `proofs/util/list_inj.v`, `proofs/concat_injective.v`, contractivity, safety, integration, cascade, blinding, and model files. |
| **Status** | ⚠️ |
| **Gap** | Current CI automates Coq compilation, admitted-marker rejection, axiom coverage checking, proof-object hashing, and artifact upload. It does not yet resolve proof-to-code refinement, extraction equivalence, independent reproducibility of `.vo` hashes outside GitHub Actions, or the PDF-requested TLA+/Apalache gate. |

### P0-8: Pinned Rust toolchain

| Field | Value |
|-------|-------|
| **PDF §** | §8.1 (p. 23, provisional) |
| **PDF quote** | `Rust 1.75.0 (pinned)`; ADR-005 revises the active repository pin to Rust 1.95.0 because Rust 1.75.0 is no longer viable with the current Cargo lockfile format. |
| **Code** | `rust-toolchain.toml` pins `channel = "1.95.0"`; `.github/workflows/ci.yml`, `.github/workflows/platform-determinism.yml`, and `.github/workflows/fuzz-smoke.yml` install that pinned toolchain and run `scripts/verify_rust_toolchain.sh` to print and check `rustc --version --verbose`. |
| **Test / Vector** | Local reproducibility verification passed for `cargo +1.95.0 build --workspace --no-default-features --locked --offline`; cross-ISA vectors must still run under the pinned toolchain before genesis lock. |
| **Proof** | — |
| **Status** | ⚠️ |
| **Gap** | Rust 1.95.0 is pinned and locally build-verified. Cross-ISA CI must provide the final multi-target reproducibility evidence before genesis lock. |

### P0-9: Genesis hash

| Field | Value |
|-------|-------|
| **PDF §** | §2.1 (pp. 2–3, provisional), §10.1 (pp. 25–26, provisional), Appendix E (p. 31, provisional) |
| **PDF quote** | `genesis_hash = "SHA3-256:<computed_hash>"` / `sha3-256sum genesis.toml` |
| **Code** | `GENESIS_CONSTANTS.toml` records a recomputable pre-lock artifact-set digest and explicitly marks `genesis_status = "provisional"` with `deployment_authoritative = false`. `scripts/verify_genesis_hash.sh` recomputes the digest from `spec/genesis-artifacts.txt`. |
| **Test / Vector** | `.github/workflows/ci.yml` and `.github/workflows/genesis-guard.yml` run `./scripts/verify_genesis_hash.sh`. |
| **Proof** | — |
| **Status** | ❌ |
| **Gap** | Terminal gate. The current hash is provisional and not deployment-authoritative. It cannot be locked until the normative PDF is committed, every provisional quote/page reference in this file plus `docs/errata/` and `docs/adr/` is verified against that PDF, ERR-001 and relevant ADRs are resolved, full encoding is defined, and cross-ISA vectors pass. |

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

## P1+ — Deferred Work Items

| ID | PDF § | Topic | Deferred because |
|----|-------|-------|------------------|
| P1-1 | §3.1 (pp. 5–6, provisional) | Multi-primitive cascade verification | Crypto integration phase. |
| P1-2 | §4.3 (p. 10, provisional) | Dual-path verification | Requires cascade verification. |
| P1-3 | §3.5 (pp. 8–9, provisional) | Crypto agility schedule | Requires cascade selection and vector coverage. |
| P1-4 | §5 | Hardware abstraction and deployment tiers | PAL implementation phase. |
| P1-5 | §5 | Hosted PAL nondeterminism boundary | Minimal hosted runtime now implemented; cross-ISA replay artifacts deferred. |
| P1-6 | §6 | Obfuscation VM | Later subsystem phase. |
| P1-7 | §7 | Clone protocol | Later subsystem phase. |
| P1-8 | PDF-SILENT | Runtime optimization track | Scheduled by `docs/adr/ADR-006-runtime-optimization-track.md`; implementation is deferred until parity and benchmark gates exist. |
| P2-1 | §9.1 (pp. 24–25, provisional) | `vm_correctness` proof | Depends on obfuscation VM. |
| P2-2 | §9.1 (pp. 24–25, provisional) | `decoy_state_identity` proof | Depends on obfuscation VM. |
