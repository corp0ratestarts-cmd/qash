# QASH Protocol — Developer Roadmap

This document captures the complete project direction from current state through
v1.1 in maximum technical detail. It is the authoritative reference for any
developer picking up this codebase.

**Last updated:** 2026-05-19  
**Current state:** Phase 3 in progress (item 10 remaining before v1.0 tag)

---

## Repository orientation

| Path | Purpose | Domain |
|------|---------|--------|
| `crates/consensus/` | `no_std`, no-alloc consensus core — the only proof-eligible, replay-invariant code | **A** |
| `crates/pal/` | Platform Abstraction Layer — `Time`, `Net`, `Attest`, `Halt` traits; `std` feature gates a hosted stub | **B** |
| `src/` | Hosted binary (`qash`) + stub modules | **B** |
| `proofs/` | Coq formal proofs, coverage matrix, STATUS ledger | — |
| `fuzz/` | honggfuzz harnesses for 6 Domain A targets | — |
| `artifacts/` | Benchmark results, build attestations, proof artifact index | — |
| `docker/` | Pinned build environment (`Dockerfile.build`) | — |
| `docs/` | Protocol specs, ADRs, threat model, refinement pipeline | — |
| `GENESIS_CONSTANTS.toml` | Immutable genesis parameters — treat as append-only | — |

**Domain A rules** (enforced by CI clippy and type system): no `unsafe`, no `f32`/`f64`,
no `usize`/`isize` in state fields or wire arithmetic, no `HashMap`, no wall clock,
no OS entropy, all arithmetic checked (overflow → `Halt::absorbing_reset()`).

---

## Completed work (merged to `main`)

### Phase 1 — Pre-genesis lock

| # | Item | PR | Key files |
|---|------|----|-----------|
| 1 | Discharge open proof obligations | #52, #58 | `proofs/cascade/cascade_collision_resistance.v`, `proofs/cascade/it_mac_forgery_bound.v`, `proofs/blinding/blinding_non_interference.v`, `proofs/COVERAGE.md` |
| 2 | Proof CI pipeline (Coq install, admit-marker rejection, axiom guard, `.vo` hash recording) | #57 | `.github/workflows/ci.yml`, `scripts/check_axiom_coverage.sh`, `proofs/artifact-index/` |
| 3 | Deep Domain A module audit | earlier PRs | `crates/consensus/src/fixed_point.rs`, `encoding.rs`, `lyapunov.rs`, `hash.rs`, `transaction.rs` |
| 4 | Fuzz coverage expansion | #56 | `fuzz/fuzz_targets/encoding_fuzz.rs`, `transition_fuzz.rs` (halt-trigger mode), 6 total targets |

### Phase 2 — Operational hardening

| # | Item | PR | Key files |
|---|------|----|-----------|
| 5 | PAL Host integration tests (Domain B → A boundary) | #59 | `crates/pal/tests/integration.rs` |
| 6 | Performance characterization (Criterion benchmarks) | #60 | `crates/consensus/benches/epoch_transition.rs`, `artifacts/benchmarks/2026-05-19T-epoch-transition-x86_64.txt` |
| 7 | Adversarial simulation suite (23 tests, 10 scenarios) | #61 | `crates/consensus/tests/adversarial_simulation.rs` |

**Adversarial simulation findings (SIM-3):** A single validator with D ≥ 50,003
(out of SCALE=1,000,000) can halt consensus regardless of quorum size. This is
by design — the Lyapunov weight W_D=400,000 means divergence above the threshold
triggers the safety halt. Documented in the test file with exact threshold derivation.

### Phase 3 — Assurance hardening

| # | Item | PR | Key files |
|---|------|----|-----------|
| 8 | Reproducible builds (Docker + attestation CI) | #62 | `docker/Dockerfile.build`, `scripts/attest_release.sh`, `.github/workflows/release-attestation.yml` |
| 9 | Proof-to-code refinement | #63 (open) | `proofs/model/RefinementStatement.v`, `proofs/model/Extract.v`, `docs/refinement.md` |

---

## Remaining work before v1.0 reference tag

### Phase 3 item 10 — Multi-compiler differential testing

**Goal:** Build the consensus crate under two independent compiler configurations,
run identical input corpus through both, assert state roots match. Confirms that
the byte-identical build invariant extends to alternative code-generation paths
and eliminates compiler-specific UB as a source of non-determinism.

**Deliverables:**

1. `.github/workflows/multi-compiler-diff.yml` — scheduled CI workflow (weekly,
   like `platform-determinism.yml`) with two jobs:

   - **job `rustc-llvm`**: standard `cargo build --release` with `CARGO_INCREMENTAL=0`
   - **job `rustc-cranelift`**: builds with `rustc -Z codegen-backend=cranelift`
     (nightly required; gated on toolchain availability)
   - Alternative if cranelift is unavailable on stable: use two LLVM optimization
     configurations (`-C opt-level=0` vs `-C opt-level=3`) and assert state roots match

2. `scripts/run_differential_corpus.sh` — runs the canonical test vector suite against
   both builds and diffs the state-root outputs. Inputs: `proofs/model/vectors.json`
   (10 vectors). Pass: all 10 roots match across both compiler configurations.

3. `artifacts/differential/` — archival directory for diff results (parallel to
   `artifacts/attestations/`), recorded on each main-branch merge.

**Critical constraint:** Both builds must use the same `rust-toolchain.toml`-pinned
Rust version. The cranelift path uses nightly and is therefore advisory
(non-blocking CI), not a required gate. The opt-level differential uses stable
and IS a required gate.

**Implementation notes:**
- `RUSTFLAGS="-C opt-level=0"` vs `RUSTFLAGS="-C opt-level=3"` are both on stable
- State roots must be identical regardless of opt level (determinism property)
- If they differ: that is a compiler bug or UB in Domain A code — halt and investigate
- The `cranelift` path exercises a completely different code generator; if roots
  differ that signals UB in the MIR (undefined behaviour the LLVM backend was hiding)

---

## Phase milestone: v1.0 reference tag

Once Phase 3 item 10 is merged to `main` and all CI gates are green:

```bash
git tag v1.0-reference main
git push origin v1.0-reference
```

This tag marks the stable pre-v1.1 snapshot. All v1.1 work branches from `main`
after this point. The tag is never moved.

**CI state at tag time (expected):**
- `build-x86_64`: green
- `test-determinism`: green (includes `verify_two_stage_build.sh`)
- `clippy-lint`: green
- `supply-chain`: green (`cargo deny check`)
- `proofs`: green (18 PROVED, 4 CI-VERIFIED, 3 AXIOM, 2 PLACEHOLDER, 0 Admitted)
- `fuzz-smoke`: green (6 targets, 30s each)
- `platform-determinism`: green (x86_64, aarch64, riscv64gc matching state roots)
- `release-attestation`: green (byte-identical two-stage build, manifest archived)

---

## v1.1 Protocol Implementation

All v1.1 work is on feature branches that PR into `main`. Each PR must include:
- A `[genesis-change-acknowledged]` token in the body if it modifies `GENESIS_CONSTANTS.toml`
- Updated `proofs/COVERAGE.md` entries for any new proof obligations
- Passing CI on all required gates

**Dependency order matters.** Items are listed in the order they must be implemented.

---

### 2-A: CI Toolchain Stabilization

**Branch:** `codex/v1.1-ci-toolchain`  
**Depends on:** nothing (can start immediately after v1.0 tag)

**Goal:** Expand CI matrix to include aarch64 and riscv64gc as first-class targets
alongside x86_64, not just in the weekly `platform-determinism.yml`.

**Changes:**
- `.github/workflows/ci.yml`: add matrix for `aarch64-unknown-linux-gnu` and
  `riscv64gc-unknown-linux-gnu` in the `build-x86_64` and `test-determinism` jobs
  using `cross` or `qemu-user-static`
- `rust-toolchain.toml`: already pins 1.95.0 — no change needed
- Verify `cargo build --target aarch64-unknown-linux-gnu` and
  `cargo build --target riscv64gc-unknown-linux-gnu` both compile without warnings

**Gate:** CI passes on all three ISAs for every PR.

---

### 2-B: Envelope Primitives and Causal Ordering

**Branch:** `codex/v1.1-envelope-primitives`  
**Depends on:** 2-A (CI matrix must include cross targets before adding new types)

**New files:**
- `crates/consensus/src/envelope.rs`
- `crates/consensus/src/causal_order.rs`
- Update `crates/consensus/src/lib.rs` to export both

**`envelope.rs` — `Envelope` struct:**
```rust
pub struct Envelope {
    // v1.0-compatible fields
    pub payload:      [u8; 256],   // fixed-size, no Vec
    pub validator_id: u32,
    pub epoch:        u64,
    // v1.1 additions
    pub epoch_seed:     [u8; 32],
    pub sort_key:       [u8; 32],
    pub cascade_health: u32,
    pub version:        u32,       // 0x1000 = v1.0, 0x1100 = v1.1
}
```

All fields: fixed-size arrays only. No `Vec`, no `String`. Domain A rules apply.

**`causal_order.rs` — `compute_sort_key`:**
```rust
pub fn compute_sort_key(
    epoch_seed: &[u8; 32],
    shard_id:   u32,
    envelope_hash: &[u8; 32],
) -> [u8; 32] {
    // H_domain("CausalOrder", epoch_seed ∥ shard_id.to_be_bytes() ∥ envelope_hash)
    h_domain(DomainTag::CausalOrder,
             &[epoch_seed.as_slice(),
               &shard_id.to_be_bytes(),
               envelope_hash.as_slice()].concat())
}
```

Reuses existing `h_domain()` from `hash.rs`. Requires adding `DomainTag::CausalOrder`
to the `DomainTag` enum in `hash.rs`.

**New `GENESIS_CONSTANTS.toml` fields** (append-only, requires `[genesis-change-acknowledged]`):
```toml
[protocol]
version = "1.1.0"
compat_version_floor = "1.0.0"
```

**Tests required:**
- KAT vector for `compute_sort_key` with known inputs → expected 32-byte output
- `proptest`: sort order is stable (same inputs → same key, deterministic)
- `proptest`: distinct `(epoch_seed, shard_id, hash)` → distinct sort keys
  (collision resistance; probabilistic, not proved)

**Proof obligation:** File a new COVERAGE.md row for "Causal ordering determinism"
with status `CI-VERIFIED` (test-vector-verified, no Coq proof needed for v1.1).

---

### 2-C: Epoch Skew Validation

**Branch:** `codex/v1.1-epoch-semantics`  
**Depends on:** 2-B (needs `Envelope` struct)

**Goal:** Reject envelopes with epochs too far in the past or future.

**Changes in `crates/consensus/src/transition.rs`:**
```rust
pub fn validate_envelope_epoch(
    envelope_epoch: u64,
    genesis_epoch:  u64,
    current_epoch:  u64,
    skew_bound:     u64,
) -> Result<(), HaltReason> {
    if envelope_epoch < genesis_epoch {
        return Err(HaltReason::DecodeInvalid);
    }
    let max_future = current_epoch.checked_add(skew_bound)
        .ok_or(HaltReason::EpochOverflow)?;
    if envelope_epoch > max_future {
        return Err(HaltReason::DecodeInvalid);
    }
    Ok(())
}
```

**New `GENESIS_CONSTANTS.toml` fields:**
```toml
[epoch.timing]
# existing: epoch_ms, max_control_loop_ms
epoch_skew_bound = 1     # Δ: max future epochs to accept
```

**Tests:** past-epoch rejection, future-epoch rejection at skew+1, acceptance at skew.

---

### 2-D: Cascade Health Tracking

**Branch:** `codex/v1.1-cascade-health`  
**Depends on:** 2-B (needs `Envelope.cascade_health` field)

**Goal:** Track protocol health across consecutive epochs; gate finality on
sustained cascade health ≥ threshold.

**State changes in `crates/consensus/src/transition.rs`** (or new `epoch.rs`):
- Add `cascade_health: u32` to `EpochState` (or to a per-shard sub-state)
- Each epoch: `cascade_health = min(cascade_health.checked_add(1)?, depth)`
  when the cascade condition holds; reset to 0 if the condition breaks
- Finality gate: when `current_epoch > compatibility_window && cascade_health < health_threshold`,
  stall (liveness concern, not a halt — no `HaltReason` set)

**Lyapunov weight extension in `crates/consensus/src/lyapunov.rs`:**
```rust
// v1.1: add cascade health term
// V_total += cascade_health_factor * (health_threshold - cascade_health)
// This creates pressure toward cascade_health = health_threshold
```

**New `GENESIS_CONSTANTS.toml` fields:**
```toml
[cascade]
depth = 8
health_threshold = 8
compatibility_window = 100     # epochs before cascade health is required
cascade_health_factor = 50000  # weight in Lyapunov sum (FixedPoint)
```

**Tests:**
- `cascade_health` increments from 0 to 8, then saturates
- Reset to 0 on condition break
- Boundary test at `health_threshold - 1` (no stall) vs `< health_threshold` (stall)
- Checked arithmetic: `cascade_health.checked_add(1)` at u32::MAX wraps to halt

---

### 2-E: Lineage Compression (Skip-List)

**Branch:** `codex/v1.1-lineage-skiplist`  
**Depends on:** 2-B

**Goal:** Replace unbounded parent-list history with a bounded O(log N) skip-list.

**New file `crates/consensus/src/lineage.rs`:**
```rust
pub const SKIPLIST_DEPTH: usize = 10;  // covers 2^10 = 1024 ancestors

pub struct SkipListHeader {
    /// Ancestor hashes at exponential depths: [1, 2, 4, 8, ..., 2^(SKIPLIST_DEPTH-1)]
    pub commitment_hashes: [[u8; 32]; SKIPLIST_DEPTH],
}

impl SkipListHeader {
    /// Verify that target_hash is an ancestor within 2^SKIPLIST_DEPTH steps.
    pub fn verify_ancestor(&self, target_hash: &[u8; 32]) -> bool {
        self.commitment_hashes.iter().any(|h| h == target_hash)
    }

    /// Build a new header by advancing one epoch.
    pub fn advance(&self, new_hash: [u8; 32], prior_headers: &[Self]) -> Self { ... }
}
```

Domain A constraint: `[[u8;32]; SKIPLIST_DEPTH]` is fixed-size, no `Vec`.

**Tests:** KAT for 3-epoch chain; `verify_ancestor` returns true for known ancestor,
false for unknown.

---

### 2-F: Version Gating and Compatibility Window

**Branch:** `codex/v1.1-version-gating`  
**Depends on:** 2-D (needs `compatibility_window` constant), 2-B (needs `Envelope.version`)

**Changes in `crates/consensus/src/transition.rs`:**
```rust
pub const PROTOCOL_VERSION_V1_0: u32 = 0x1000;
pub const PROTOCOL_VERSION_V1_1: u32 = 0x1100;

// In advance_epoch, after epoch validation:
if state.epoch > genesis_params.compatibility_window
    && envelope.version < PROTOCOL_VERSION_V1_1 {
    return Err(HaltReason::IncompatibleVersion);
}
```

**New `HaltReason` variant:**
```rust
pub enum HaltReason {
    // ... existing variants ...
    IncompatibleVersion = 0x08,
}
```

**Tests:**
- `axiom_all_halt_reasons_roundtrip` updated to include `IncompatibleVersion`
- Transition rejects v1.0 envelope at epoch 101 (after `compatibility_window = 100`)
- Transition accepts v1.0 envelope at epoch ≤ 100

---

### 2-G: ML-KEM-768 PQC KEM (Domain B only)

**Branch:** `codex/v1.1-pqc-kem`  
**Depends on:** nothing (Domain B, no Domain A changes)

**This does NOT touch Domain A at all.** It is entirely in `crates/pal/` or a new
`crates/crypto/`.

**Dependency:** `ml-kem = "0.2"` (or `pqcrypto-mlkem`) — feature-gated `#[cfg(feature = "pqc")]`.

**New file `crates/pal/src/crypto/kem.rs`:**
```rust
#[cfg(feature = "pqc")]
pub struct MlKem768Kem {
    encapsulation_key: ml_kem::EncapsulationKey768,
}

#[cfg(feature = "pqc")]
impl MlKem768Kem {
    pub fn keygen() -> (Self, ml_kem::DecapsulationKey768) { ... }
    pub fn encapsulate(&self) -> ([u8; 32], ml_kem::Ciphertext768) { ... }
}

// Hybrid X25519 + ML-KEM-768 "X-Wing" combiner:
// shared_secret = SHA3-256(x25519_ss ∥ mlkem_ss ∥ x25519_pk ∥ mlkem_ct)
pub fn xwing_combine(
    x25519_ss: &[u8; 32],
    mlkem_ss:  &[u8; 32],
    x25519_pk: &[u8; 32],
    mlkem_ct:  &[u8],
) -> [u8; 32] { ... }
```

**`Cargo.toml` addition (crates/pal):**
```toml
[features]
pqc = ["ml-kem"]

[dependencies]
ml-kem = { version = "0.2", optional = true }
```

**Tests:** KAT encapsulate/decapsulate round-trip; X-Wing combiner matches
test vector from the X-Wing draft specification.

---

### 2-H: FIPS and Data Protection (Domain B only)

**Branch:** `codex/v1.1-fips-compliance`  
**Depends on:** nothing (Domain B only)

**Changes:**
- RNG: use `hmac-drbg` crate (NIST SP 800-90A HMAC-DRBG) in `crates/pal/src/rng.rs`
  instead of direct OS entropy; feature-gate: `#[cfg(feature = "fips-rng")]`
- TLS: add config validation in the hosted binary that rejects SSLv3 / TLS 1.0 / TLS 1.1;
  document TLS 1.2+ requirement in `docs/threat_model/`
- Logging: ensure no raw public keys or IP addresses are emitted to logs;
  use `sha3_256(pk)[..8]` as a log-safe pseudonym
- New file: `docs/compliance/fips_compliance.md` — maps each FIPS 140-3 requirement
  to the code path that addresses it

**No Domain A changes.** All `unsafe` in PAL is already under audit.

---

### 2-I: Formal Proofs for v1.1 Properties

**Branch:** `codex/v1.1-proofs`  
**Depends on:** 2-B, 2-C, 2-D, 2-F (needs stable interfaces)

**New Coq files in `proofs/`:**

1. `proofs/ordering/causal_ordering.v`:
   - Theorem: `(epoch, sort_key)` total ordering is deterministic
   - Proof: reduces to SHA3-256 preimage resistance for sort-key distinctness
   - Uses `h_domain` from `cascade/cascade_determinism.v` as the hash primitive

2. `proofs/ordering/compatibility_window.v`:
   - Theorem: during compatibility window (epoch ≤ 100), both v1.0 and v1.1
     envelopes yield the same state roots under the same transition function
   - Proof: the version field is not read during the compatibility window, so
     `advance_epoch` is identical for both versions in that range

**`proofs/COVERAGE.md` additions:**
```
| Causal ordering determinism       | §v1.1 | PROVED | causal_ordering.v         | src/causal_order.rs  | causal_order::tests::* |
| Compatibility window equivalence  | §v1.1 | PROVED | compatibility_window.v    | src/transition.rs    | (replay corpus) |
```

**CI addition:** add `ordering/causal_ordering.v` and `ordering/compatibility_window.v`
to the tier-2 compilation list in `ci.yml`.

---

### 2-J: Semantic Closure (Compile-time Domain Gating)

**Branch:** `codex/v1.1-semantic-closure`  
**Depends on:** 2-B (needs the new types to gate)

**Goal:** Prevent Domain B values from flowing into Domain A computations
at compile time, not just by convention.

**Changes in `crates/consensus/src/lib.rs`:**
```rust
#![deny(clippy::disallowed_types)]

// Domain A marker trait — zero-sized, no methods.
// All Domain A types impl this; Domain B types must not.
pub trait DomainA: Sized {}

// Capability token: marks a value as originating from Domain B.
// Cannot be passed to functions expecting Domain A inputs without
// explicit unwrap at the boundary (an audit point).
pub struct CapToken<T>(T);

impl<T> CapToken<T> {
    // Only constructible in Domain B (crates/pal or src/).
    // The `pub(crate)` restriction enforces the boundary.
    pub fn unwrap_at_boundary(self) -> T { self.0 }
}
```

**`crates/consensus/.cargo/config.toml`** (or workspace `Cargo.toml`):
```toml
[target.'cfg(any())'.dependencies]
# Deny HashMap in consensus crate
```

Or via `clippy.toml`:
```toml
disallowed-types = [
  { path = "std::collections::HashMap", reason = "Use BTreeMap for deterministic iteration" },
  { path = "std::time::Instant", reason = "Domain A must not read wall clock" },
  { path = "std::time::SystemTime", reason = "Domain A must not read wall clock" },
]
```

**`check_state_invariants` function** (in `transition.rs`):
```rust
pub fn check_state_invariants(state: &EpochState) -> Result<(), HaltReason> {
    // Scale bounds
    for v in &state.validators[..state.validator_count as usize] {
        if v.divergence.raw() > SCALE || v.divergence.raw() < 0 { ... }
        if v.conflict.raw()   > SCALE || v.conflict.raw()   < 0 { ... }
        if v.slash_accum.raw() < 0 { ... }
    }
    // Halt monotonicity: if halted, halt_reason must be non-None
    // (already enforced by enum, this is belt-and-suspenders)
    Ok(())
}
```

Called at every transition admission point.

---

### 2-K: Replay Corpus and v1.1 Conformance Tests

**Branch:** `codex/v1.1-replay-corpus`  
**Depends on:** all Domain A changes (2-B through 2-F)

**Deliverables:**

1. `tests/vectors/vectors.v1.1.json` — 50-epoch replay corpus:
   - Genesis state (v1.1 format with `cascade_health`, `version`)
   - Mixed v1.0 and v1.1 envelopes across the compatibility window
   - Expected state roots for each epoch
   - At least one halt scenario (version rejection after epoch 100)

2. `crates/consensus/tests/v1_1_replay.rs` — vector runner:
   ```rust
   #[test]
   fn v1_1_replay_corpus() {
       let corpus = include_str!("../../../tests/vectors/vectors.v1.1.json");
       // For each entry: reconstruct state, apply envelopes, assert state root
   }
   ```

3. `scripts/replay_test.sh` — CI-runnable replay script:
   ```bash
   cargo test -p qash-consensus v1_1_replay -- --nocapture
   cargo test --target aarch64-unknown-linux-gnu -p qash-consensus v1_1_replay
   cargo test --target riscv64gc-unknown-linux-gnu -p qash-consensus v1_1_replay
   ```

4. Update `platform-determinism.yml` to include v1.1 test suite in the cross-ISA check.

---

## Implementation order summary

```
Phase 3 item 10: multi-compiler differential testing
  → git tag v1.0-reference

v1.1 (feature branches):
  2-A (CI toolchain — cross-ISA matrix)
  → 2-B (envelope primitives + causal ordering)
  → 2-C (epoch skew validation)           ←┐ can be parallel
  → 2-D (cascade health tracking)         ←┘
  → 2-E (lineage skip-list)               ← can be parallel with 2-C/2-D
  → 2-F (version gating)                  ← requires 2-B + 2-D
  → 2-G (ML-KEM-768)                      ← parallel, Domain B only
  → 2-H (FIPS compliance)                 ← parallel, Domain B only
  → 2-I (formal proofs)                   ← after 2-B..2-F are stable
  → 2-J (semantic closure)                ← after 2-B
  → 2-K (replay corpus)                   ← after all Domain A changes merged
```

---

## GENESIS_CONSTANTS.toml additions for v1.1 (append-only)

Every addition requires `[genesis-change-acknowledged]` in the PR body.

```toml
[epoch.timing]
# existing: epoch_ms = 500, max_control_loop_ms = 450
epoch_skew_bound = 1           # Δ: max future epochs to accept (2-C)

[cascade]
depth = 8                      # cascade depth (2-D)
health_threshold = 8           # epochs of sustained health required (2-D)
compatibility_window = 100     # epochs before v1.0 envelopes rejected (2-D, 2-F)
cascade_health_factor = 50000  # Lyapunov weight for health term (2-D)

[protocol]
version = "1.1.0"              # wire version (2-B)
compat_version_floor = "1.0.0" # lowest accepted version during window (2-F)
```

---

## Proof obligation tracking for v1.1

Every new transaction type or state-transition variant requires a COVERAGE.md row
**before** the implementation PR is merged. The axiom guard CI script enforces this:
any new `Axiom` declaration in `proofs/` that doesn't appear in `proofs/COVERAGE.md`
fails CI.

Current COVERAGE.md state (after Phase 3 item 9):
- **PROVED:** 18
- **CI-VERIFIED:** 4
- **AXIOM:** 3 (AX-3/SHA3, Blinding PRF, AX2_rust_refinement)
- **PLACEHOLDER:** 2 (TH-10 cascade collision, IT-MAC forgery bound)
- **Total:** 27

v1.1 additions will add: causal ordering (PROVED), compatibility window (PROVED),
and potentially new axioms for ML-KEM-768 security (Domain B, separate file).

---

## Key invariants that are never changed

These are fixed constraints inherited from v1.0. No v1.1 work may violate them:

1. `GENESIS_CONSTANTS.toml` is append-only. Modifying an existing field defines
   a new network.
2. Domain A (`crates/consensus/`) forbids: `unsafe`, `f32`/`f64`,
   `usize`/`isize` in state fields, `HashMap`, wall clock, OS entropy, unchecked arithmetic.
3. All arithmetic overflow in Domain A triggers absorbing halt.
4. Cross-ISA replay invariance (TH-7) is a non-negotiable CI gate.
5. Every new transaction type requires a filed proof obligation before
   implementation merges.
6. `proofs/COVERAGE.md` is the authoritative proof obligation ledger.
   The `check_axiom_coverage.sh` script enforces consistency with the Coq source.

---

## How to contribute

```bash
# Clone and build
git clone https://github.com/corp0ratestarts-cmd/qash
cd qash
cargo build --workspace --no-default-features

# Run all tests
cargo test --workspace --no-default-features

# Run Coq proofs
cd proofs
coqc -Q . QASH crypto_game_framework.v
coqc -Q . QASH util/list_inj.v
coqc -Q . QASH contractivity/lyapunov_stability.v
# ... (see ci.yml for full compilation order)

# Run fuzz smoke test (30s each target)
cd fuzz && cargo fuzz run cascade_fuzz -- -max_total_time=30

# Reproduce build attestation
cargo build --release --no-default-features
bash scripts/attest_release.sh

# Run inside Docker (fully pinned environment)
docker build -t qash-build -f docker/Dockerfile.build .
docker run --rm -v "$PWD":/workspace qash-build cargo test --workspace --no-default-features
```

Before opening a PR:
- All CI gates must pass (`cargo test`, `cargo clippy`, `cargo deny check`)
- Any new `Axiom` declaration in `proofs/` must appear in `proofs/COVERAGE.md`
- Any modification of `GENESIS_CONSTANTS.toml` requires `[genesis-change-acknowledged]`
  in the PR body
- Domain A changes require a proof obligation row in `proofs/COVERAGE.md`
- Benchmark results for performance-sensitive changes go in `artifacts/benchmarks/`
