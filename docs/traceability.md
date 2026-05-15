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
| **Code** | `crates/consensus/src/lyapunov.rs` computes `V_convergence = α·D + β·C` and `phi_safety = γ·max_slash`; `crates/consensus/src/transition.rs` checks `delta_window > epsilon` and triggers halt. |
| **Test / Vector** | — |
| **Proof** | `proofs/contractivity/lyapunov_stability.v` exists, but is not CI-verified. |
| **Status** | 🔶 |
| **Blocking** | `docs/errata/ERR-001-lyapunov-monotone-term.md` is accepted with the two-function partition. `docs/adr/ADR-001-phi-safety-accumulator.md` and `docs/adr/ADR-002-phi-safety-aggregation.md` must still resolve the safety threshold and aggregation rule before the implementation can be validated. |

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
| **Code** | `.github/workflows/platform-determinism.yml` invokes `scripts/verify_cross_isa_identity.sh`, which runs `qash-vector-runner` sequentially for `x86_64`, `aarch64`, and `riscv64gc` and diffs outputs. |
| **Test / Vector** | `tests/vectors/vectors.v1.json`; `tests/vector-runner`. |
| **Proof** | — |
| **Status** | ⚠️ |
| **Gap** | The script and vector runner exist, but non-native QEMU execution was not verified in this environment. The state-root vector is code-derived until ADR-003 defines full state encoding. |

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
| **Proof** | PDF §9.1 names `concat_injective`; repository proof content is not CI-verified. |
| **Status** | ⚠️ |
| **Gap** | Code appears byte-identical to the PDF pseudocode and now has vector coverage, but proof CI still needs to prove or map the named theorem. |

### P0-7: Formal verification CI integration

| Field | Value |
|-------|-------|
| **PDF §** | §9.3 (p. 25, provisional) |
| **PDF quote** | `cd proofs && make all` / `apalache-mc check --length=100 tla/QASHConsensus.tla` / `test -f proofs/concat_injective.vo` / `test -f proofs/vm_correctness.vo` |
| **Code** | `.github/workflows/ci.yml` has proof and TLA smoke jobs. |
| **Test / Vector** | — |
| **Proof** | `proofs/Makefile`, `proofs/_CoqProject`, `proofs/contractivity/lyapunov_stability.v`, `proofs/util/list_inj.v`, and `tla/QASHConsensus.tla` exist. |
| **Status** | ⚠️ |
| **Gap** | CI is wired for Coq proof compilation and a TLA+ smoke model, but the local environment did not have Coq/Apalache installed. The TLA+ model is explicitly a stub and must be replaced before genesis lock. |

### P0-8: Pinned Rust toolchain

| Field | Value |
|-------|-------|
| **PDF §** | §8.1 (p. 23, provisional) |
| **PDF quote** | `Rust 1.75.0 (pinned)` |
| **Code** | `rust-toolchain.toml` pins `channel = "1.75.0"`; `.github/workflows/ci.yml` and `.github/workflows/platform-determinism.yml` use that pinned toolchain. |
| **Test / Vector** | Cross-ISA vectors must run under the pinned toolchain. |
| **Proof** | — |
| **Status** | ⚠️ |
| **Gap** | The pin exists and ADR-005 is accepted. Local verification used an already-installed newer toolchain because Rust 1.75.0 could not be downloaded from this environment. |

### P0-9: Genesis hash

| Field | Value |
|-------|-------|
| **PDF §** | §2.1 (pp. 2–3, provisional), §10.1 (pp. 25–26, provisional), Appendix E (p. 31, provisional) |
| **PDF quote** | `genesis_hash = "SHA3-256:<computed_hash>"` / `sha3-256sum genesis.toml` |
| **Code** | `GENESIS_CONSTANTS.toml` has `genesis_hash = "SHA3-256:PLACEHOLDER"`. |
| **Test / Vector** | — |
| **Proof** | — |
| **Status** | ❌ |
| **Gap** | Terminal gate. The hash cannot be locked until the normative PDF is committed, ERR-001 and relevant ADRs are resolved, full encoding is defined, and cross-ISA vectors pass. |

---

## P1+ — Deferred Work Items

| ID | PDF § | Topic | Deferred because |
|----|-------|-------|------------------|
| P1-1 | §3.1 (pp. 5–6, provisional) | Multi-primitive cascade verification | Crypto integration phase. |
| P1-2 | §4.3 (p. 10, provisional) | Dual-path verification | Requires cascade verification. |
| P1-3 | §3.5 (pp. 8–9, provisional) | Crypto agility schedule | Requires cascade selection and vector coverage. |
| P1-4 | §5 | Hardware abstraction and deployment tiers | PAL implementation phase. |
| P1-5 | §6 | Obfuscation VM | Later subsystem phase. |
| P1-6 | §7 | Clone protocol | Later subsystem phase. |
| P2-1 | §9.1 (pp. 24–25, provisional) | `vm_correctness` proof | Depends on obfuscation VM. |
| P2-2 | §9.1 (pp. 24–25, provisional) | `decoy_state_identity` proof | Depends on obfuscation VM. |
