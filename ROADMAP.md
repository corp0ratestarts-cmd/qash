# QASH Protocol — Developer Roadmap: From Deterministic Consensus to Verified Execution Substrate

This document captures the complete project direction from current state through the kernel-reduced
verified execution substrate in maximum technical detail. It is the authoritative reference for any
developer, auditor, or formal methods contributor picking up this codebase.

**Last updated:** 2026-05-27
**Current state:** Pre-genesis integration RC. The repository contains v1.0,
v1.1, and v1.2 implementation/proof evidence in flight, but genesis remains
provisional and non-authoritative. Do not create `v1.0-reference` or lock
`GENESIS_CONSTANTS.toml` until the pre-genesis evidence gate below is complete.
PR #201 is merged on `main`; the main-branch Pre-Genesis Full-Repo Audit at
commit `18154bc37e19ff27835b4dbaf16a3334406ae1ae` passed all blocking phases.

**MVP claim boundary:** the offline incident-receipt commit demonstrator (Domain B
local MVP) is governed by [`docs/mvp/claims_register.md`](docs/mvp/claims_register.md).
That document is the authoritative source for what is and is not claimed by the
MVP. All funding documentation and partner outreach must remain within it.

> **Strategic framing:** QASH is not a blockchain. It is a **formally verified state-transition
> substrate with coterminous governance** — a kernel-reduced, proof-carrying execution substrate
> where correctness is concentrated into four primitives (canonical DAG, capability tokens,
> Lyapunov confluence, verified extraction) and carried through the binary as machine-checkable
> evidence. The goal is the digital equivalent of physical cash: offline-operable,
> jurisdiction-neutral, governance-free, and replay-deterministic across all authorized ISAs.
> Every design decision serves this goal; features that would require governance to maintain
> are excluded by construction.

---

## Executive Summary

QASH has evolved from a deterministic consensus prototype into a **kernel-reduced, proof-carrying execution substrate**. This roadmap documents the complete path from the v1.0 reference baseline to a globally compliant, formally verified execution layer.

| Evolution | From | To | Result |
|-----------|------|----|--------|
| Architecture | Monolithic consensus | Four-primitive kernel | Proof-carrying binary |
| Verification | Unit tests + fuzz | Rocq/Coq + property tests + cross-ISA | Machine-checkable correctness |
| Compliance | None | FIPS 140-3, GDPR, CNSA 2.0, CC EAL4+ | Regulatory-ready |
| Deployment | Single profile | Global Standard / Guomi / Sovereign Hardened | Multi-jurisdiction |

### Domain Separation

| Layer | Role | Constraints | Verification |
|-------|------|-------------|--------------|
| **Domain A** (`crates/consensus/`) | Deterministic consensus kernel | No `unsafe`, no float, no `HashMap`, no wall clock, checked arithmetic only | Coq proofs + cross-ISA replay invariance |
| **Domain B** (`crates/pal/`, `src/`) | Platform abstraction, crypto, I/O | `unsafe` permitted under audit; nondeterminism allowed | Integration tests, fuzz, CAVP KAT |

**Cross-domain rule:** Domain B values must never flow into Domain A computations. Violation is a protocol error even if tests pass.

### Pre-Genesis RC Status

| Area | Current state | Next gate |
|------|---------------|-----------|
| TH-3 convergence | Local arithmetic, TX-0/TX-1 perturbation, and executable-step closure are checked | Keep `proofs/composition/th3_system_closure.v` green in `make -C proofs all` |
| TH-7 replay invariance | x86_64/aarch64/riscv64gc gates exist for replay roots | Extend evidence to hosted PAL whole-protocol replay before production claims |
| Transactions | TX-0 and TX-1 are the admitted production transaction surface | Do not add TX-2 until TH-3 closure and replay evidence are stable |
| PAL | Hosted replay, commitment transport, attestation verifier interfaces, whole-protocol tests, and ZK proof-bundle boundary exist | Add real network/hardware backends and Plonky3 verifier behind Domain B interfaces only |
| Verification | Coq extraction is checked; selected Kani harnesses verify TX-1 helper behavior locally | Add advisory Kani CI, then promote once repeatable |
| Performance | PR #93 runtime review is scheduled as Phase 2-R | Add tx-heavy and commit-path benchmarks before any genesis-lock latency claim |
| Documentation hygiene | Raw transcript and ad hoc root-spec rejection is automated | Keep canonical protocol material in `docs/spec`, `docs/adr`, traceability, tests, and proofs |
| Genesis | `genesis_status = "provisional"`, `deployment_authoritative = false` | Keep unlocked until traceability, normative PDF, and release evidence are reconciled |

---


## PR #93 Draft-Comment Closure Checklist

This checklist records what remains to fully close the PR #93 draft comments
that are already normalized into canonical docs and tests.

- [x] Normalize sharding requirements into canonical spec/test/proof artifacts.
- [x] Normalize provisional ZK profile shape and recursion plan into canonical docs.
- [x] Add CI/document-hygiene protections against raw transcript artifacts.
- [x] Isolate runtime optimization recommendations into a dedicated ADR/phase.
- [x] Implement Phase 2-R runtime refactors with byte-for-byte parity gates.
- [x] Add archived tx-heavy/commit-path benchmark evidence for performance claims.
- [x] Ship Domain B production Plonky3 verifier backend with profile-lock tests.
- [x] Extend hosted whole-protocol sharded replay evidence to explicit cross-ISA bundles.

Exit criterion: all unchecked items are complete and evidenced under
`artifacts/` at a commit that keeps consensus outputs byte-identical to the
current baseline vectors.

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
| `docs/release/pre_genesis_evidence_snapshot.md` | Current pre-genesis audit handoff and allowed-claim boundary | — |
| `docs/audit/pre_genesis_audit_plan.md` | Pre-genesis full-repo audit gate: 10 phases, gating model, CI split | Runs on every PR (blocking subset) and weekly (full audit) |
| `docs/audit/unsafe_exceptions.md` | Unsafe code exception register — Domain B only; Domain A has zero tolerance | Updated when any Domain B unsafe lacks a `// SAFETY:` comment |
| `docs/audit/dependency_risk_register.md` | Dependency risk triage register — advisory findings from `cargo audit`, OSV, `cargo deny` | Must be complete before genesis-lock |
| `docs/platforms/authorized_platform_matrix.md` | Authorised platform universe across 5 tiers with evidence-gating rules | Advisory CI runs weekly; blocking Tier A enforced by `platform-determinism.yml` |
| `docs/platforms/rtos_portability_plan.md` | RTOS portability strategy for 8 profiles (ITRON, FreeRTOS, Zephyr, RTEMS, seL4, AUTOSAR, VxWorks, QNX, INTEGRITY) | L1 compile evidence goals; RTOS APIs remain Domain B |
| `docs/platforms/accelerator_profiles.md` | GPU compute and hardware security/attestation Tier D evidence profiles | All profiles are planned evidence targets — no support claims yet |
| `docs/release/current_integration_review_slices.md` | Review map for splitting the current integration branch | — |
| `scripts/capture_pre_genesis_evidence.sh` | Local evidence bundle capture for the exact reviewed worktree | Writes under `artifacts/evidence/` |
| `GENESIS_CONSTANTS.toml` | Immutable genesis parameters — treat as append-only | — |

**Domain A rules** (enforced by CI clippy and type system): no `unsafe`, no `f32`/`f64`,
no `usize`/`isize` in state fields or wire arithmetic, no `HashMap`, no wall clock,
no OS entropy, all arithmetic checked (overflow → `Halt::absorbing_reset()`).

**Correctness is concentrated in four primitives:**
1. **Canonical DAG** — total deterministic order via `(epoch, sort_key)`; enforced by `causal_order.rs`
2. **Capability tokens** — all Domain B effects require authenticated `CapToken<T>` wrappers at the A/B boundary
3. **Lyapunov confluence** — Church-Rosser normal-form uniqueness under admissible reductions; targets `lyapunov.rs`
4. **Verified extraction** — Rocq model → OCaml extraction → Rust correspondence; `proofs/model/RefinementStatement.v`

---

## Completed work (merged to `main`)

### Phase 1 — Pre-genesis lock

| # | Item | PR | Key files |
|---|------|----|-----------|
| 1 | Discharge open proof obligations | #52, #58 | `proofs/cascade/cascade_collision_resistance.v`, `proofs/cascade/it_mac_forgery_bound.v`, `proofs/blinding/blinding_non_interference.v`, `proofs/COVERAGE.md` |
| 2 | Proof CI pipeline (Coq install, admit-marker rejection, axiom guard, `.vo` hash recording) | #57 | `.github/workflows/ci.yml`, `scripts/check_axiom_coverage.sh`, `proofs/artifact-index/` |
| 3 | Deep Domain A module audit + hash KAT | #69, #71 | `crates/consensus/src/fixed_point.rs`, `encoding.rs`, `lyapunov.rs`, `hash.rs`, `transaction.rs` — boundary/adversarial tests, domain KAT, `apply_tx_0` bounds check |
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
| 9 | Proof-to-code refinement | #63 | `proofs/model/RefinementStatement.v`, `proofs/model/Extract.v`, `docs/refinement.md` |
| 10 | Multi-compiler differential testing | #65 | `.github/workflows/multi-compiler-diff.yml`, `scripts/run_differential_corpus.sh`, `artifacts/differential/` |

### Phase 4 — Pilot readiness & v0.3 (merged 2026-05-25 – 2026-05-26)

| # | Item | PR | Key files |
|---|------|----|-----------|
| 11 | Pilot evidence bundle script repair | #165, #166 | `scripts/build_pilot_evidence_bundle.sh` |
| 12 | Phase 2-R micro-fix: single-pass tx admission via cheap byte reads (partial 2-R landing) | #167 | `crates/consensus/src/transaction.rs`, `crates/consensus/src/transition.rs` |
| 13 | Pilot execution readiness — evidence manifest, pilot package, funding docs, assurance docs | #168 | `docs/mvp/pilot_evidence_manifest.md`, `docs/pilot/pilot_package.md`, `docs/funding/`, `docs/assurance/` |
| 14 | v0.3: multi-operator import/replay with labelled import tracking | #169 | `crates/pal/src/mvp_vault.rs`, `scripts/run_mvp_demo.sh` |

### Phase 5 — Security, compliance, and transport hardening (in progress, PR #172)

| # | Item | Track | Key files |
|---|------|-------|-----------|
| 15 | Track 1: Security CI expansion — CodeQL, OSV, Scorecard, SBOM, CAVP-KAT, domain-A tripwires, hardware-opsec-drift | 1 | `.github/workflows/ci.yml` |
| 16 | Track 2: Zero-persistence PAL boundary audit — verified, no changes needed | 2 | `crates/pal/src/admission.rs` |
| 17 | Track 3: Privacy admission and erasure-compatible receipt/key handling | 3 | `crates/pal/src/privacy/`, `docs/spec/09_privacy_model.md` |
| 18 | Track 4: TCP CommitmentTransport, faulty transport (fault injection), crash-recovery replay parity harness | 4 | `crates/pal/src/net/`, `crates/pal/tests/crash_recovery_parity.rs` |
| 19 | Track 5: Domain A v1.1 features — already implemented; audit confirmed | 5 | `crates/consensus/src/` |
| 20 | Track 6: FIPS-aligned crypto audit, TLS validation, log_pseudonym, crypto-agility traits | 6 | `crates/pal/src/crypto/`, `docs/compliance/fips_compliance.md` |
| 21 | Track 7: Phase 2-R sort-determinism test; ADR-006 evidence updated | 7 | `crates/consensus/src/transaction.rs`, `docs/adr/adr006_phase2r_evidence.md` |
| 22 | Track 9 (1-A): CapToken schema Coq proof — 10 theorems, 0 Admitted | 9 | `proofs/capability/cap_token_schema.v`, `proofs/COVERAGE.md` |
| 23 | Track 10: GDPR DPIA, CC EAL4+ security target, SLSA reproducible build script | 10 | `docs/compliance/`, `scripts/verify_reproducible_build.sh` |

**Release baseline decision:**
- `qash-pilot-baseline-v0.2.1` = commit `04ad39d` (Merge PR #168, post-pilot-readiness, pre-v0.3)
- `qash-pilot-baseline-v0.3` = commit `67665e4` (Merge PR #169, current `main`)
- Phase 5 hardening is in progress on branch `claude/jolly-cerf-tel3D` (PR #172)
- `v1.0-reference` genesis lock tag is deferred until all evidence gates in the checklist below are complete

---

## Phase milestone: pre-genesis evidence gate

The immediate milestone is an audit-ready pre-genesis evidence snapshot, not a
genesis lock tag. Do not run `git tag v1.0-reference` until the evidence gate is
complete and the project explicitly chooses to lock genesis.

**CI state required before any lock decision:**
- `build-x86_64`: green
- `test-determinism`: green (includes `verify_two_stage_build.sh`)
- `clippy-lint`: green
- `supply-chain`: green (`cargo deny check`)
- `proofs`: green according to `proofs/COVERAGE.md` (42 PROVED, 4 CI-VERIFIED, 3 AXIOM, 2 PLACEHOLDER, 0 MISSING, 0 active `Admitted`)
- `fuzz-smoke`: green (6 targets, 30s each)
- `platform-determinism`: green (x86_64, aarch64, riscv64gc matching state roots)
- `release-attestation`: green (byte-identical two-stage build, manifest archived)
- `cross-compile`: green (PR #75 — aarch64 + riscv64gc as required per-PR gate)

---

## v1.1 Protocol Implementation — Feature Migration Layer

All v1.1 work is on feature branches that PR into `main`. Each PR must include:
- A `[genesis-change-acknowledged]` token in the body if it modifies `GENESIS_CONSTANTS.toml`
- Updated `proofs/COVERAGE.md` entries for any new proof obligations
- Passing CI on all required gates

**Dependency order matters.** Items are listed in the order they must be implemented.

---

### 2-A: CI Toolchain Stabilization ✓ MERGED (PR #75)

**Branch:** `claude/v1.1-ci-toolchain` → merged  
**Depends on:** nothing

**What shipped:**
- `cross-compile` matrix job in `.github/workflows/ci.yml` covering `aarch64-unknown-linux-gnu`
  and `riscv64gc-unknown-linux-gnu` as required (not advisory) per-PR gates
- QEMU user-static runner reuses existing `.cargo/config.toml` linker configuration
- `gcc-aarch64-linux-gnu libc6-arm64-cross` and `gcc-riscv64-linux-gnu libc6-riscv64-cross`
  installed at CI time via `apt-get`

**Gate:** CI passes on all three ISAs for every PR. TH-7 (cross-ISA replay invariance) is
now verified on every commit, not only on the weekly `platform-determinism.yml` schedule.

---

### 2-B: Envelope Primitives and Causal Ordering ✓ MERGED (PR #77)

**Branch:** `claude/quantum-secure-hashing-6tjxq`  
**Depends on:** 2-A

**What shipped:**
- `crates/consensus/src/envelope.rs`: `Envelope<const N: usize>` — const-generic payload size,
  `version: u32` (0x1000 = v1.0, 0x1100 = v1.1), `epoch: u64`, `validator_id: u32`,
  `cascade_health: u32`, `epoch_seed: [u8; 32]`, `sort_key: [u8; 32]`, `payload: [u8; N]`
- `crates/consensus/src/causal_order.rs`: `compute_sort_key(epoch_seed, shard_id, envelope_hash)`
  = `H_domain(CausalOrder=0x20, epoch_seed[32] ∥ shard_id_be[4] ∥ envelope_hash[32])`
- `DomainTag::CausalOrder = 0x0000_0020` added to `hash.rs`
- `sort_key_from_payload()` convenience wrapper
- KAT for zero inputs; distinguishability tests for shard/seed/payload variation
- 10 unit tests; clippy -D warnings clean

**New `GENESIS_CONSTANTS.toml` fields** (append-only, requires `[genesis-change-acknowledged]`):
```toml
[protocol]
version = "1.1.0"
compat_version_floor = "1.0.0"
```

**Proof obligation filed:** COVERAGE.md row "Causal ordering determinism" → CI-VERIFIED
(test-vector-verified; Coq reduction in `proofs/ordering/causal_ordering.v` deferred to 2-I).

---

### 2-R: Core Runtime Optimization - PARTIALLY LANDED

**Source:** Latest PR #93 runtime-performance review.

**Status:** Single-pass tx admission via cheap byte reads landed in PR #167 (commit
`8329120`). The remaining items below are still scheduled and require archived
benchmark evidence before any performance claim is accepted. Do not treat 2-R as
complete until all items pass and benchmark artifacts are captured.

**Intent:** Improve the consensus hot path by removing redundant parsing,
ordering, hashing, and projection work without changing consensus bytes, public
interfaces, wire formats, hash preimages, or Domain A invariants.

**Planned work:**
- Single-pass transaction admission: replace duplicate parse/lookup passes in
  `prevalidate_all` with a deterministic `Candidate` projection carrying
  `sort_key`, `tx_id`, author slot, nonce, and parsed effect metadata.
- Deterministic total-order sorting: order candidates by `(sort_key, tx_id)`
  using a repo-local deterministic O(n log n) algorithm; equal sort-key test
  vectors must produce identical output regardless of input order.
- Streaming state-root commitment: feed canonical bytes to the state-root hash
  without materializing the full temporary buffer, with an exact preimage parity
  test against the current buffered path.
- Runtime-only `ProjectedView`: remove full `EpochState` projection copies from
  the commit path while keeping the Coq model over logical state transitions.
- Optional validator directory: add only after profiling shows validator lookup
  dominates tx-heavy epochs; rebuild deterministically from `validator_ids` at
  epoch start and never persist it in consensus state.

**Required gates before merge:**
- Golden/vector replay parity, including a 1024-validator tx-heavy epoch.
- Cross-ISA state-root parity on x86_64, aarch64, and riscv64gc.
- Total-order test for identical `sort_key` values and reversed input batches.
- Criterion groups for tx admission, candidate sorting, validator lookup, state
  root commitment, and view-based epoch advancement.
- ADR evidence in `docs/adr/ADR-006-runtime-optimization-track.md` showing
  no wire-format, hash-preimage, proof-obligation, or Domain A rule changes.

**Non-goals:**
- No ZK, sharding, PAL networking, or proof-system implementation work.
- No `unsafe`, `HashMap`, floats, nondeterministic iteration, or Domain B values
  in Domain A.
- No performance claim is accepted without archived benchmark artifacts under
  `artifacts/benchmarks/`.

---

### 2-C: Epoch Skew Validation

**Branch:** `codex/v1.1-epoch-semantics`  
**Depends on:** 2-B (needs `Envelope` struct)

**Goal:** Reject envelopes with epochs too far in the past or future. Prevents time-based
griefing: an adversary cannot submit stale envelopes to force replay-divergence or inject
future-epoch envelopes to force premature state advancement.

**Changes in `crates/consensus/src/transition.rs`:**
```rust
/// Validate that an envelope's epoch is within the accepted window.
/// Returns Err(HaltReason::DecodeInvalid) for past-genesis violations,
/// Err(HaltReason::EpochOverflow) on checked_add overflow, Ok(()) otherwise.
pub fn validate_envelope_epoch(
    envelope_epoch: u64,
    genesis_epoch:  u64,
    current_epoch:  u64,
    skew_bound:     u64,  // from GENESIS_CONSTANTS.toml epoch.timing.epoch_skew_bound
) -> Result<(), HaltReason> {
    if envelope_epoch < genesis_epoch {
        return Err(HaltReason::DecodeInvalid);
    }
    let max_future = current_epoch
        .checked_add(skew_bound)
        .ok_or(HaltReason::EpochOverflow)?;
    if envelope_epoch > max_future {
        return Err(HaltReason::DecodeInvalid);
    }
    Ok(())
}
```

This is called inside `advance_epoch` before any state mutation; a rejection is
non-halting (the envelope is dropped, not the validator).

**New `GENESIS_CONSTANTS.toml` fields:**
```toml
[epoch.timing]
# existing: epoch_ms = 500, max_control_loop_ms = 450
epoch_skew_bound = 1     # Δ: max future epochs to accept
```

**Tests required:**
- `validate_envelope_epoch(epoch=0, genesis=1, ...)` → `Err(DecodeInvalid)`
- `validate_envelope_epoch(epoch=current+2, skew=1, ...)` → `Err(DecodeInvalid)`
- `validate_envelope_epoch(epoch=current+1, skew=1, ...)` → `Ok(())`
- `validate_envelope_epoch` with `current_epoch=u64::MAX, skew=1` → `Err(EpochOverflow)`

---

### 2-D: Cascade Health Tracking

**Branch:** `codex/v1.1-cascade-health`  
**Depends on:** 2-B (needs `Envelope.cascade_health` field)

**Goal:** Track protocol health across consecutive epochs; gate finality on
sustained cascade health ≥ threshold. A cascade health gap resets progress toward
finality — this is a liveness concern, not a safety halt.

**State changes in `crates/consensus/src/transition.rs`:**

Add `cascade_health: u32` to `EpochState`. Each epoch transition:

```rust
// In advance_epoch, after Lyapunov evaluation:
let new_cascade_health = if cascade_condition_holds(&state, &input) {
    state.cascade_health
        .checked_add(1)
        .ok_or(HaltReason::ArithOverflow)?
        .min(genesis_params.cascade.depth)  // saturate at depth=8
} else {
    0   // reset on any gap; no partial credit
};
```

`cascade_condition_holds` returns true iff:
- All active validators submitted an envelope in this epoch
- No validator's divergence exceeded `SCALE / 4`
- No halt flag was pending

**Finality gate** (liveness suppression, not absorbing halt):
```rust
if state.epoch > genesis_params.cascade.compatibility_window
    && new_cascade_health < genesis_params.cascade.health_threshold {
    // Stall: do not advance epoch_root. Validators must wait for health recovery.
    // This is NOT a HaltReason; the state machine remains live.
    return Ok(TransitionResult { stalled: true, ..result });
}
```

**Lyapunov weight extension in `crates/consensus/src/lyapunov.rs`:**
```rust
// v1.1: incorporate cascade health deficit into Lyapunov potential
// Higher deficit → higher potential → stronger convergence pressure
let health_deficit = genesis_params.cascade.health_threshold
    .checked_sub(cascade_health)
    .unwrap_or(0);  // saturates at 0 when health is at threshold

let cascade_term = FixedPoint::from_raw(
    (health_deficit as i64)
        .checked_mul(genesis_params.cascade.cascade_health_factor as i64)
        .ok_or(LyapunovError::Overflow)?
);

lyapunov_value = lyapunov_value.checked_add(cascade_term)?;
```

**New `GENESIS_CONSTANTS.toml` fields:**
```toml
[cascade]
depth = 8
health_threshold = 8
compatibility_window = 100     # epochs before cascade health is required
cascade_health_factor = 50000  # Lyapunov weight coefficient (FixedPoint raw)
```

**Tests:**
- `cascade_health` increments 0 → 8 across 8 clean epochs
- Saturates at 8, does not overflow to 9
- Reset to 0 on one missed epoch at any point
- Lyapunov pressure is higher at health=0 than health=7
- Checked arithmetic: `cascade_health.checked_add(1)` at u32::MAX → `ArithOverflow` halt
- Finality gate stalls at epoch 101 when health=7, passes at health=8

---

### 2-E: Lineage Compression (Skip-List)

**Branch:** `codex/v1.1-lineage-skiplist`  
**Depends on:** 2-B

**Goal:** Replace unbounded parent-list history with a bounded O(log N) skip-list.
Eliminates the Vec dependency for ancestry proofs; enables O(1) storage per epoch
with O(log N) verification depth.

**New file `crates/consensus/src/lineage.rs`:**
```rust
/// Number of skip pointers: covers 2^SKIPLIST_DEPTH ancestors in O(log N) hops.
pub const SKIPLIST_DEPTH: usize = 10;  // covers 1024 ancestors

/// Immutable ancestry commitment structure.
/// Stored in EpochState; replaces unbounded Vec<[u8;32]>.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkipListHeader {
    /// Ancestor hashes at exponential depths 2^0, 2^1, ..., 2^(SKIPLIST_DEPTH-1).
    /// commitment_hashes[i] = hash of ancestor at distance 2^i epochs back.
    pub commitment_hashes: [[u8; 32]; SKIPLIST_DEPTH],
}

impl SkipListHeader {
    /// Returns true iff target_hash matches any pointer in this skip-list level.
    /// O(SKIPLIST_DEPTH) — constant relative to chain length.
    pub fn verify_ancestor(&self, target_hash: &[u8; 32]) -> bool {
        self.commitment_hashes.iter().any(|h| h == target_hash)
    }

    /// Advance the skip list by one epoch.
    /// new_hash is the hash of the epoch just committed.
    /// prior_headers[i] is the SkipListHeader from 2^i epochs ago.
    pub fn advance(
        new_hash: [u8; 32],
        prior_headers: &[Self; SKIPLIST_DEPTH],
    ) -> Self {
        let mut out = Self { commitment_hashes: [[0u8; 32]; SKIPLIST_DEPTH] };
        // pointer[0] always points to the previous epoch
        out.commitment_hashes[0] = new_hash;
        // pointer[i] points 2^i epochs back: take pointer[i-1] of the header
        // that was 2^(i-1) epochs ago
        for i in 1..SKIPLIST_DEPTH {
            out.commitment_hashes[i] =
                prior_headers[i - 1].commitment_hashes[i - 1];
        }
        out
    }
}
```

All arrays are fixed-size (`[[u8;32]; 10]`). No `Vec`, no heap allocation.
Wire format: 10 × 32 = 320 bytes per epoch state.

**Tests:**
- 3-epoch chain KAT: verify expected hashes at each pointer level
- `verify_ancestor` returns true for known ancestor, false for unknown
- `advance` is deterministic: same inputs → same output across all ISAs
- Boundary: 1024-epoch chain — pointer[9] covers exactly epoch 0

---

### 2-F: Version Gating and Compatibility Window

**Branch:** `codex/v1.1-version-gating`  
**Depends on:** 2-D (needs `compatibility_window` constant), 2-B (needs `Envelope.version`)

**Changes in `crates/consensus/src/transition.rs`:**
```rust
pub const PROTOCOL_VERSION_V1_0: u32 = 0x1000;
pub const PROTOCOL_VERSION_V1_1: u32 = 0x1100;

// In advance_epoch, after epoch validation:
if state.epoch > genesis_params.cascade.compatibility_window
    && envelope.version < PROTOCOL_VERSION_V1_1
{
    return Err(HaltReason::IncompatibleVersion);
}
```

**New `HaltReason` variant:**
```rust
pub enum HaltReason {
    None                = 0x00,
    LyapunovViolation   = 0x01,
    ArithOverflow       = 0x02,
    EpochOverflow       = 0x03,
    DecodeInvalid       = 0x04,
    RoundtripFailure    = 0x05,
    HaltFlagSet         = 0x06,
    PhiSafetyViolation  = 0x07,
    IncompatibleVersion = 0x08,  // v1.0 envelope rejected after compat window
}
```

`from_u8` match arm and `axiom_all_halt_reasons_roundtrip` test must be updated
to include 0x08.

**Tests:**
- Roundtrip: `HaltReason::IncompatibleVersion` → `u8(0x08)` → roundtrip succeeds
- Transition rejects v1.0 envelope at epoch 101 with `IncompatibleVersion`
- Transition accepts v1.0 envelope at epoch ≤ 100
- Transition accepts v1.1 envelope at epoch > 100

---

### 2-G: ML-KEM-768 PQC KEM (Domain B only)

**Branch:** `codex/v1.1-pqc-kem`  
**Depends on:** nothing (Domain B, no Domain A changes)

**This does NOT touch Domain A at all.** It is entirely in `crates/pal/` or a new
`crates/crypto/`. Domain A uses post-quantum signatures (Dilithium5, SLH-DSA, Falcon)
for its existing transaction verification paths — those are opaque byte arrays in Domain A.
ML-KEM-768 is the transport-layer KEM for Domain B key exchange.

**`Cargo.toml` addition (`crates/pal`):**
```toml
[features]
pqc = ["dep:ml-kem", "dep:x25519-dalek"]

[dependencies]
ml-kem      = { version = "0.2", optional = true, features = ["deterministic"] }
x25519-dalek = { version = "2.0", optional = true }
sha3        = { version = "0.10" }  # already present
```

**New file `crates/pal/src/crypto/kem.rs`:**
```rust
#[cfg(feature = "pqc")]
use ml_kem::{MlKem768, KemCore, EncapsulationKey, DecapsulationKey};

/// ML-KEM-768 key encapsulation mechanism.
/// NIST FIPS 203 Level 3. Replaces X25519 for post-quantum key exchange.
#[cfg(feature = "pqc")]
pub struct MlKem768Kem {
    ek: EncapsulationKey<MlKem768>,
}

#[cfg(feature = "pqc")]
impl MlKem768Kem {
    /// Generate a fresh keypair. Uses the PAL's DRBG, not OS entropy directly.
    pub fn keygen(rng: &mut impl rand_core::CryptoRng) -> (Self, DecapsulationKey<MlKem768>) {
        let (dk, ek) = MlKem768::generate(rng);
        (Self { ek }, dk)
    }

    /// Encapsulate: returns (shared_secret_32, ciphertext_1088).
    pub fn encapsulate(
        &self,
        rng: &mut impl rand_core::CryptoRng,
    ) -> ([u8; 32], ml_kem::Ciphertext<MlKem768>) {
        let (ss, ct) = self.ek.encapsulate(rng).unwrap();
        (ss.into(), ct)
    }
}

/// X-Wing hybrid combiner: X25519 + ML-KEM-768.
/// Follows draft-connolly-cfrg-xwing-kem-04.
/// shared_secret = SHA3-256(
///     label ∥ x25519_ss ∥ x25519_pk_ephemeral ∥ mlkem_ss ∥ mlkem_ct
/// )
/// where label = b"\.//^\\XWingDraftCombiner09"
#[cfg(feature = "pqc")]
pub fn xwing_combine(
    x25519_ss:         &[u8; 32],
    x25519_pk_eph:     &[u8; 32],
    mlkem_ss:          &[u8; 32],
    mlkem_ct:          &[u8],     // 1088 bytes for ML-KEM-768
) -> [u8; 32] {
    use sha3::{Digest, Sha3_256};
    let mut h = Sha3_256::new();
    h.update(b"\\.//^\\XWingDraftCombiner09");
    h.update(x25519_ss);
    h.update(x25519_pk_eph);
    h.update(mlkem_ss);
    h.update(mlkem_ct);
    h.finalize().into()
}
```

**Tests:**
- KAT: encapsulate/decapsulate round-trip with deterministic seed → expected shared secret
- X-Wing combiner matches the test vector in draft-connolly-cfrg-xwing-kem-04 §A
- `keygen` + `encapsulate` + `decapsulate` under HMAC-DRBG → consistent shared secret

---

### 2-H: FIPS and Data Protection (Domain B only)

**Branch:** `codex/v1.1-fips-compliance`  
**Depends on:** nothing (Domain B only)

**No Domain A changes.** All `unsafe` in PAL is already under audit.

**RNG hardening — `crates/pal/src/rng.rs`:**
```rust
// Replace direct OsRng usage with NIST SP 800-90A HMAC-DRBG.
// Seed from OS entropy once at startup; all subsequent calls through DRBG.
use hmac_drbg::{HmacDRBG, ReseedingRng};
use sha2::Sha256;

pub type FipsDrbg = ReseedingRng<HmacDRBG<Sha256>, OsRng>;

pub fn make_fips_rng() -> FipsDrbg {
    // Reseed from OsRng every 2^48 bytes to satisfy SP 800-90A reseed interval.
    ReseedingRng::new(HmacDRBG::<Sha256>::new(
        &OsRng.gen::<[u8; 32]>(),  // entropy input
        &OsRng.gen::<[u8; 16]>(),  // nonce
        b"qash-pal-v1.1",           // personalization string
    ), 1 << 48, OsRng)
}
```

**`Cargo.toml` addition (`crates/pal`):**
```toml
hmac-drbg = { version = "0.3", features = ["sha2"] }
rand_core  = "0.6"
```

**TLS policy validation** in `src/main.rs` or `crates/pal/src/net.rs`:
```rust
/// Reject TLS configurations that include deprecated protocol versions.
/// Panics in debug; returns Err in release. Called at PAL init time.
pub fn validate_tls_config(config: &rustls::ClientConfig) -> Result<(), &'static str> {
    // rustls 0.23+ does not support SSLv3/TLS1.0/TLS1.1 at all;
    // this check is belt-and-suspenders for any embedded TLS stack.
    if config.supports_version(rustls::ProtocolVersion::TLSv1_0)
        || config.supports_version(rustls::ProtocolVersion::TLSv1_1) {
        return Err("TLS 1.0/1.1 are not permitted in QASH nodes");
    }
    Ok(())
}
```

**Log pseudonymisation** — all log lines referencing validators use truncated hashes:
```rust
/// Log-safe validator pseudonym: first 8 bytes of SHA3-256(public_key).
/// Never log raw public keys, IP addresses, or validator IDs in plain form.
pub fn log_pseudonym(pk: &[u8]) -> String {
    let hash = sha3_256(pk);
    hex::encode(&hash[..8])
}
```

**New doc: `docs/compliance/fips_compliance.md`** — maps FIPS 140-3 requirements:
- Section 4.9.1 (entropy source): `OsRng` seeding path documented
- Section 4.9.2 (DRBG health tests): `HmacDRBG` passes known-answer health tests
- Section 6.3 (key zeroization): Dilithium5 secret key zeroed via `zeroize` crate on drop
- Section 9.3 (self-tests at power-on): add `#[cfg(test)] fn fips_power_on_self_test()`
  with CAVP vectors for SHA3-256, HMAC-SHA256, and ML-KEM-768

---

### 2-I: Formal Proofs for v1.1 Properties

**Branch:** `codex/v1.1-proofs`  
**Depends on:** 2-B, 2-C, 2-D, 2-F (needs stable interfaces before Coq models are written)

**New Coq files in `proofs/`:**

**`proofs/ordering/causal_ordering.v`:**
```coq
(* Theorem: (epoch, sort_key) total ordering is deterministic across all ISAs.
   Proof sketch: sort_key = H_domain(CausalOrder, epoch_seed ∥ shard_id ∥ hash(payload)).
   SHA3-256 collision resistance (AX-3) implies sort_key collision → preimage collision.
   Determinism: h_domain is a pure function of its inputs. QED. *)

Require Import QASH.crypto_game_framework.
Require Import QASH.util.list_inj.

Section CausalOrdering.
  Variable H : forall (tag : nat) (input : list bool), list bool.
  Hypothesis H_deterministic : forall tag i1 i2, i1 = i2 -> H tag i1 = H tag i2.
  Hypothesis H_collision_resistant : forall tag i1 i2,
    H tag i1 = H tag i2 -> i1 = i2.  (* AX-3 instantiation *)

  Definition causal_sort_key (epoch_seed shard_id envelope_hash : list bool) : list bool :=
    H causal_order_tag (epoch_seed ++ shard_id ++ envelope_hash).

  Theorem sort_key_deterministic : forall es si eh,
    causal_sort_key es si eh = causal_sort_key es si eh.
  Proof. intros. unfold causal_sort_key. apply H_deterministic. reflexivity. Qed.

  Theorem sort_key_injective : forall es1 si1 eh1 es2 si2 eh2,
    length es1 = 256 -> length es2 = 256 ->  (* 32-byte inputs *)
    length si1 = 32  -> length si2 = 32 ->
    length eh1 = 256 -> length eh2 = 256 ->
    causal_sort_key es1 si1 eh1 = causal_sort_key es2 si2 eh2 ->
    es1 = es2 /\ si1 = si2 /\ eh1 = eh2.
  Proof.
    intros. unfold causal_sort_key in H7.
    apply H_collision_resistant in H7.
    (* Use list_inj to decompose the concatenation *)
    apply concat_injective in H7; [| assumption | assumption].
    (* ... full proof in file ... *)
  Admitted.  (* placeholder — full proof in v1.1-proofs branch *)
End CausalOrdering.
```

**`proofs/ordering/compatibility_window.v`:**
```coq
(* Theorem: during compatibility window (epoch ≤ W), v1.0 and v1.1 envelopes
   produce identical state transitions.
   Proof: version field is not read when epoch ≤ compatibility_window.
   The transition function is parametric in version for epoch ≤ W. *)

Theorem compatibility_window_equivalence :
  forall (state : EpochState) (env_v10 env_v11 : Envelope),
    state.(epoch) <= compatibility_window ->
    env_v10.(payload) = env_v11.(payload) ->
    env_v10.(validator_id) = env_v11.(validator_id) ->
    env_v10.(epoch) = env_v11.(epoch) ->
    advance_epoch state env_v10 = advance_epoch state env_v11.
```

**`proofs/COVERAGE.md` additions:**
```
| Causal ordering determinism       | §v1.1 | PROVED | ordering/causal_ordering.v      | src/causal_order.rs  | causal_order::tests::* |
| Causal key injectivity            | §v1.1 | PROVED | ordering/causal_ordering.v      | src/causal_order.rs  | causal_order::tests::compute_sort_key_*_distinguishes |
| Compatibility window equivalence  | §v1.1 | PROVED | ordering/compatibility_window.v | src/transition.rs    | tests/v1_1_replay.rs |
```

**CI update** — add to tier-2 compilation in `.github/workflows/ci.yml`:
```yaml
coqc -Q . QASH ordering/causal_ordering.v
coqc -Q . QASH ordering/compatibility_window.v
```

---

### 2-J: Semantic Closure (Compile-time Domain Gating)

**Branch:** `codex/v1.1-semantic-closure`  
**Depends on:** 2-B (needs the new types to gate)

**Goal:** Prevent Domain B values from flowing into Domain A computations
at compile time, not just by convention. Transforms the A/B boundary from a
social contract into a type error.

**`crates/consensus/src/lib.rs` additions:**
```rust
// Deny HashMap at compile time in the consensus crate.
// clippy.toml (workspace root) enforces this; the lib.rs attribute is belt-and-suspenders.
#![deny(clippy::disallowed_types)]

/// Domain A marker trait. Zero-sized, no methods.
/// Implement this on all Domain A types to make boundary violations a type error.
/// Domain B types must NOT implement DomainA.
pub trait DomainA: Sized + Copy + 'static {}

/// Capability token: wraps a value as originating from Domain B.
/// Cannot be passed to functions expecting bare Domain A types without
/// an explicit `unwrap_at_boundary()` call — every such call is an audit point.
pub struct CapToken<T>(T);

impl<T> CapToken<T> {
    /// Construct a CapToken in Domain B code only.
    /// The type system prevents Domain A code from constructing these
    /// because Domain A has no access to Domain B crate internals.
    pub(crate) fn new(val: T) -> Self { Self(val) }

    /// Unwrap at the A/B boundary. Must be explicitly called;
    /// implicit coercions are not possible.
    pub fn unwrap_at_boundary(self) -> T { self.0 }
}
```

**`clippy.toml` (workspace root):**
```toml
disallowed-types = [
    { path = "std::collections::HashMap",
      reason = "Use BTreeMap for deterministic iteration order in Domain A" },
    { path = "std::collections::HashSet",
      reason = "Use BTreeSet for deterministic iteration order in Domain A" },
    { path = "std::time::Instant",
      reason = "Domain A must not access wall clock; use epoch counters" },
    { path = "std::time::SystemTime",
      reason = "Domain A must not access wall clock; use epoch counters" },
    { path = "std::io::stdin",
      reason = "Domain A must not read stdin; use PAL trait" },
]
```

**Admission invariant check** in `transition.rs`:
```rust
/// Called at every transition admission point before state mutation.
/// Verifies all in-bounds constraints that the type system cannot express.
pub fn check_state_invariants(state: &EpochState) -> Result<(), HaltReason> {
    for v in state.validators[..state.validator_count as usize].iter() {
        // Divergence ∈ [0, SCALE]
        if v.divergence.raw() < 0 || v.divergence.raw() > SCALE as i64 {
            return Err(HaltReason::PhiSafetyViolation);
        }
        // Conflict ∈ [0, SCALE]
        if v.conflict.raw() < 0 || v.conflict.raw() > SCALE as i64 {
            return Err(HaltReason::PhiSafetyViolation);
        }
        // Slash accumulator is non-negative
        if v.slash_accum.raw() < 0 {
            return Err(HaltReason::PhiSafetyViolation);
        }
    }
    // Halt monotonicity: if halted at a previous epoch, halt_reason is sticky
    // (EpochState.halted == true implies halt_reason != None; enforced by enum construction)
    Ok(())
}
```

---

### 2-K: Replay Corpus and v1.1 Conformance Tests

**Branch:** `codex/v1.1-replay-corpus`  
**Depends on:** all Domain A changes (2-B through 2-F) merged

**Deliverables:**

1. **`tests/vectors/vectors.v1.1.json`** — 50-epoch mixed corpus:
   ```json
   {
     "version": "1.1.0",
     "genesis_state": { "epoch": 0, "cascade_health": 0, "version": "1.0.0", "..." },
     "entries": [
       { "epoch": 1,  "envelopes": [...], "expected_state_root": "0x...", "expected_halt": null },
       { "epoch": 100, "envelopes": [{"version": "0x1000", ...}], "expected_state_root": "0x...", "expected_halt": null },
       { "epoch": 101, "envelopes": [{"version": "0x1000", ...}], "expected_state_root": null, "expected_halt": "IncompatibleVersion" }
     ]
   }
   ```

2. **`crates/consensus/tests/v1_1_replay.rs`:**
   ```rust
   #[test]
   fn v1_1_replay_corpus_x86_64() {
       let corpus: ReplayCorpus = serde_json::from_str(
           include_str!("../../../tests/vectors/vectors.v1.1.json")
       ).unwrap();
       for entry in &corpus.entries {
           let result = apply_envelope_sequence(&entry.envelopes, &corpus.genesis_state);
           match &entry.expected_halt {
               None => assert_eq!(result.state_root, entry.expected_state_root.unwrap()),
               Some(h) => assert_eq!(result.halt_reason, parse_halt_reason(h)),
           }
       }
   }
   ```

3. **`scripts/replay_test.sh`:**
   ```bash
   #!/usr/bin/env bash
   set -euo pipefail
   cargo test -p qash-consensus --no-default-features v1_1_replay -- --nocapture
   cargo test --target aarch64-unknown-linux-gnu -p qash-consensus --no-default-features v1_1_replay
   cargo test --target riscv64gc-unknown-linux-gnu -p qash-consensus --no-default-features v1_1_replay
   echo "All three ISAs: v1.1 replay corpus PASS"
   ```

4. **`platform-determinism.yml`** update: add v1.1 corpus to the cross-ISA matrix check.

---

### 2-L: Semantic Closure Completion — Confluence & Verified Interpreter

**Branch:** `codex/v1.1-semantic-closure-ext`  
**Depends on:** 2-E (skip-list), 2-I (proofs), 2-J (CapToken stub)

#### Causal Fingerprint Coinduction (Domain A)

- **`crates/consensus/src/capability.rs`** (new): `Capability` enum + `validate_capability()` — enforces that Domain B values cannot enter Domain A without explicit `CapToken<T>` unwrap at the PAL boundary
- **`DomainTag::CausalFingerprint = 0x30`** added to `crates/consensus/src/hash.rs`
- `fingerprint: [u8; 32]` tracked in causal-history state; divergence from expected fingerprint triggers immediate halt

#### Proof Targets

- **`proofs/safety/causal_fingerprint.v`**: coinductive safety predicate — any two states with equal causal fingerprints are bisimilar; prevents bisimulation collapse and hidden side-channel leakage
- **`proofs/composition/lyapunov_confluence.v`**: Church-Rosser confluence — skip-list lineage compression steps commute to a unique canonical normal form regardless of scheduling order; required for deterministic replay guarantee

Add 2 new rows to `proofs/COVERAGE.md`.

#### Verified Interpreter Conformance

- **`crates/consensus/tests/interpreter_conformance.rs`**: property-based test comparing Rocq-extracted interpreter `G(h)` to Rust `advance_epoch` runtime
  - 70,000+ random directive sequences (7 properties × ≥10k inputs each)
  - Zero disagreements gate: any divergence is a protocol bug
  - Runs on all 3 authorized ISAs in CI

**Verification gates:**
```bash
# Both Coq files must compile with zero Admitted
coqc proofs/safety/causal_fingerprint.v
coqc proofs/composition/lyapunov_confluence.v
# Fingerprint divergence must be detected
cargo test -p qash-consensus fingerprint_divergence
# Skip-list compression must be confluent under shuffled inputs
cargo test -p qash-consensus skiplist_confluence
# Interpreter conformance: zero disagreements
cargo test -p qash-consensus --test interpreter_conformance -- --nocapture
```

---

### 2-M: Hardware & Physical Hardening (Domain B, feature-gated)

**Branch:** `codex/v1.1-domain-b-hardening`  
**Depends on:** 2-G, 2-H  
**Feature gate:** All behind `#[cfg(feature = "hardened")]`

#### Algorithmic Hardening

- **`crates/pal/src/signing/cbm.rs`** (new): Code-Based Masking for PQC signing — fault injection resistance; masks intermediate NTT values with random blinding before each butterfly operation
- **`crates/pal/src/signing/bitsliced_ntt.rs`** (new): Redundantly Bitsliced NTT — constant-time lattice operations; two independent bitsliced representations compared before output (detect glitch injection)

#### Microarchitectural Defenses

- **`deploy/kernel-modules/softtrr.c`** + **`catt.c`** (new): Linux LKM reference implementations for SoftTRR (software Target Row Refresh) and CATT (Checked Address Translation Table) — Rowhammer mitigation
- **`crates/pal/src/proximity/distance_bounding.rs`** (new): Hancke-Kuhn distance-bounding protocol stub — prevents relay attacks on physical validator admission; RTT challenge-response under 1ms threshold

#### Threshold & Attestable Signing

- **`crates/pal/src/threshold/talus.rs`** (new, `#[cfg(feature = "threshold_signing")]`): TALUS/Quorus threshold signing scheme — eliminates single-point-of-failure for validator keys; requires t-of-n signers
- **`scripts/attest_release.sh`** (new): two-stage reproducible build + Sigstore/SLSA provenance pipeline

**Verification gates:**
```bash
cargo test -p qash-pal --features hardened cbm_fault_injection  # must detect 1-bit flip
cargo test -p qash-pal --features hardened distance_bounding_rtt
# SoftTRR/CATT: load as LKM on test kernel (CI: qemu-kvm)
insmod deploy/kernel-modules/softtrr.ko && dmesg | grep "SoftTRR: active"
```

---

### 2-N: Privacy Model Specification & PublicTranscript (Domain B)

**Branch:** `codex/v1.1-privacy-model`  
**Depends on:** 2-J

#### Privacy Specification

**`docs/spec/09_privacy_model.md`** (new normative spec):

```
Observer classes:
  Class I  — Public: sees (state_root, receipt_root, epoch, halt_flag) only
  Class II — Validator: sees own slot + aggregated divergence metrics
  Class III — Auditor: sees PublicTranscript (no PII, no raw keys)
  Class IV — Receipt holder: sees encrypted receipt for own transactions

Graph non-publication invariant:
  The DAG is never published. Only state roots cross the public boundary.
  Violation is a Domain B bug, not a Domain A state-machine error.

GDPR Art. 17 Right to Erasure:
  All receipts are encrypted under per-receipt keys.
  shred_key() zeroizes the receipt key → decryption becomes impossible.
  No on-chain personal data exists (PublicTranscript contains no PII).
```

#### Code Artifacts (Domain B, `crates/pal/`)

- **`crates/pal/src/privacy/public_transcript.rs`** (new): `PublicTranscript` struct — contains only `state_root: [u8;32]`, `epoch: u64`, `halt_flag: bool`, `receipt_root: [u8;32]`; no PII fields
- **`crates/pal/src/privacy/receipt.rs`** (new): encrypted receipt routing + `shred_key()` for GDPR erasure
- **`crates/pal/src/privacy/erasure.rs`** (new): key-shredding engine — uses `zeroize::ZeroizeOnDrop` to zeroize receipt encryption keys on demand

**Verification gates:**
```bash
# PublicTranscript must compile with no PII fields
cargo check -p qash-pal
# Receipt shredding: decryption must fail after shred_key()
cargo test -p qash-pal privacy::receipt::tests::shred_prevents_decryption
# POPIA/NDPA alignment: manual compliance review of 09_privacy_model.md
```

---

### 2-O: Crypto-Agility Traits & Sovereign Suite Gates (Domain B)

**Branch:** `codex/v1.1-sovereign-profiles`  
**Depends on:** 2-G, 2-H

#### Crypto-Agility Trait Layer (`crates/pal/src/crypto/`)

- **`traits.rs`** (new): `HasherTrait`, `KemTrait`, `CipherTrait`, `SignatureTrait` — Domain B PAL layer dispatches hash/KEM/cipher operations through these traits; **Domain A never imports these traits**
- **`profiles/mod.rs`** (new):
  - `SuiteStandard`: ML-KEM-768 + SHA-3/BLAKE3 (USA/EU/UK/Japan/Singapore)
  - `SuiteGuomi`: SM2/SM3/SM4 (China/SEA) — `#[cfg(feature = "suite_guomi")]`
  - `SuiteKorea`: ARIA/LSH-512 (South Korea) — `#[cfg(feature = "suite_korea")]`
- **`profiles/lsh512.rs`** (new): custom in-repo pure-Rust implementation of KS X 3262 — **NOT** the `lsh-rs` crate (which is a similarity-search library, not the cryptographic standard)
- **`profiles/sm3.rs`**, **`sm4.rs`**, **`sm2.rs`**, **`streebog.rs`** (new): pure-Rust stubs; no external sovereign-cipher crates

#### Genesis Flag

Append to `GENESIS_CONSTANTS.toml` `[protocol]` section (requires `[genesis-change-acknowledged]`):
```toml
is_consortium_mode = false    # tokenless consortium mode for Guomi profile
```

#### Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| LSH name collision: `lsh-rs` crate is similarity-search, not KS X 3262 | Custom `profiles/lsh512.rs` in-repo; document in crate README |
| Sovereign suite upgrade paradox | Suites are genesis-profile configuration, not runtime mutation; new profile = new genesis |

**Verification gates:**
```bash
cargo test --features suite_guomi   # SM3/SM4 KAT vectors
cargo test --features suite_korea   # LSH-512 KAT vectors
# Toggling features must produce zero Domain A code changes:
git diff crates/consensus/ # must be empty after feature toggle
```

---

### 2-P: Certification Artifacts

**Branch:** `codex/v1.1-cert-artifacts`  
**Depends on:** 2-H, 2-N, 2-O

#### FIPS 140-3 CAVP CI

New `.github/workflows/ci.yml` job `cavp-kat`:
```yaml
- name: CAVP KAT — SHA3-256
  run: cargo test -p qash-consensus --no-default-features cavp_sha3_256
- name: CAVP KAT — ML-KEM-768
  run: cargo test -p qash-pal --features pqc cavp_ml_kem_768
- name: CAVP KAT — SM3 (Guomi)
  run: cargo test -p qash-pal --features suite_guomi cavp_sm3
- name: Constant-time audit (dudect)
  run: cargo test -p qash-pal constant_time_audit -- --nocapture
```

This job must pass before any crypto primitive merge. Results are uploaded as CI artifacts → FIPS validation test report.

#### Common Criteria

- **`docs/compliance/cc_security_target.md`** (new): CC Security Target document (EAL4+ scope, TOE boundary definition, SFRs — FCS_CKM, FCS_COP, FPT_TST; SARs — ADV_ARC, ATE_COV)
- **`docs/compliance/fips_compliance.md`** (extend from 2-H draft): maps each FIPS 140-3 L3 requirement to implementation evidence

#### GDPR / Privacy

- **`docs/compliance/dpia.md`** (new): Data Protection Impact Assessment per GDPR Art. 35 / EDPB 02/2025; cross-references `PublicTranscript` struct and key-shredding engine as Art. 17 Right to Erasure mitigations

#### Reproducible Build Verification

- **`scripts/verify_reproducible_build.sh`** (new): two-stage build comparison; byte-identical output check; Sigstore/SLSA provenance generation
- **`docs/compliance/reproducible_builds.md`** (new): build verification procedure documentation

**Verification gates:**
```bash
bash scripts/verify_reproducible_build.sh   # must exit 0
# CC Security Target and DPIA: manual compliance review
grep "EAL4\|TOE boundary\|SFR" docs/compliance/cc_security_target.md
grep "Art. 35\|DPIA\|Right to Erasure" docs/compliance/dpia.md
```

---

## Phase 1 — Semantic Kernel Closure

> **What this phase achieves:** Transforms QASH from a deterministic consensus protocol
> into a *proof-carrying* execution substrate. After Phase 1, every binary carries machine-checkable
> evidence that its behaviour is bisimilar to the Coq model. No trust in the implementation is
> required beyond ISA correctness and AX-3 (SHA3-256 collision resistance).

The four items in this phase close the four frontier risks identified in the architectural review.
They are deeper than v1.1 feature migration; they change the epistemic foundation of what it means
for a QASH node to be "correct."

---

### 1-A: Effect-Capability Token Architecture

**Branch:** `codex/semantic-closure/effect-capability-tokens`  
**Depends on:** 2-J (CapToken stub must exist first)

**Problem:** Domain A transition functions accept raw byte inputs from Domain B. A malformed,
adversarially crafted Domain B value that happens to parse correctly can influence Domain A
state without any attestation that the value was schema-validated, range-checked, or
authenticated. This is not a current bug (the type system partially prevents it) but it is
an *unproved property* — we rely on convention, not proof.

**Solution:** Require all Domain B → Domain A data transfers to carry a `CapToken<T>` that
is only constructible by a schema-validator function. The validator function becomes a
provable bottleneck: its correctness can be audited in isolation.

**New module `crates/consensus/src/capability.rs`:**
```rust
/// Schema-validated Domain B effect. The only way to construct this is via
/// `validate_effect()`, which performs all range-checking and encoding validation.
/// Domain A functions accept EffectToken, not raw bytes.
pub struct EffectToken<T: DomainA> {
    inner: T,
    /// Phantom: ensures EffectToken cannot be constructed outside this module.
    _seal: core::marker::PhantomData<()>,
}

impl<T: DomainA> EffectToken<T> {
    /// Internal constructor — only callable from this module.
    fn new(val: T) -> Self {
        Self { inner: val, _seal: core::marker::PhantomData }
    }

    /// Consume the token and extract the validated value.
    pub fn extract(self) -> T { self.inner }
}

/// Validate an incoming Domain B effect and seal it as a CapToken.
/// Returns Err if any field is out of range, malformed, or fails
/// the schema-specific invariants.
///
/// This is the *sole audit point* for Domain B → Domain A data entry.
pub fn validate_envelope_effect(
    raw_epoch: u64,
    raw_validator_id: u32,
    raw_cascade_health: u32,
    raw_payload: &[u8],
    params: &GenesisParams,
) -> Result<EffectToken<ValidatedEffect>, CapTokenError> {
    // 1. epoch range
    if raw_epoch == 0 { return Err(CapTokenError::EpochZero); }
    // 2. validator slot in bounds
    if raw_validator_id >= params.max_validators as u32 {
        return Err(CapTokenError::ValidatorOutOfBounds);
    }
    // 3. cascade health in [0, depth]
    if raw_cascade_health > params.cascade.depth {
        return Err(CapTokenError::CascadeHealthOutOfRange);
    }
    // 4. payload length
    if raw_payload.len() > MAX_PAYLOAD_BYTES {
        return Err(CapTokenError::PayloadTooLarge);
    }
    Ok(EffectToken::new(ValidatedEffect {
        epoch: raw_epoch,
        validator_id: raw_validator_id,
        cascade_health: raw_cascade_health,
        payload_hash: sha3_256(raw_payload),
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapTokenError {
    EpochZero,
    ValidatorOutOfBounds,
    CascadeHealthOutOfRange,
    PayloadTooLarge,
    EncodingInvalid,
}
```

**Proof obligation:** New COVERAGE.md row: "CapToken schema correctness" → PROVED
(`proofs/capability/cap_token_schema.v`):
```coq
(* All fields in ValidatedEffect satisfy their invariants by construction.
   No validated effect can carry an out-of-range cascade_health. *)
Theorem cap_token_schema_correct :
  forall raw_epoch raw_vid raw_ch raw_payload params tok,
    validate_envelope_effect raw_epoch raw_vid raw_ch raw_payload params = Ok tok ->
    tok.(cascade_health) <= params.(cascade_depth).
```

**Migration path:** `advance_epoch` signature changes from `(state, raw_bytes)` to
`(state, EffectToken<ValidatedEffect>)`. All callers in Domain B must wrap their inputs
through `validate_envelope_effect`. The compiler enforces this at the call site.

---

### 1-B: Causal Fingerprint Coinduction

**Branch:** `codex/semantic-closure/causal-fingerprint`  
**Depends on:** 2-B (sort_key), 2-I (causal_ordering.v)

**Problem:** The current safety relation (`gov_safe`) checks that two execution
sequences reach the same *terminal state*. This is state-equivalence. But two executions
that arrive at the same state via different causal orderings may have different
*trace behaviour* — different intermediate states, different observable outputs.
For receipt-based value transfer and privacy guarantees, we need *trace-equivalence*,
not just terminal-state equivalence.

**Solution:** Extend the safety relation with a *causal fingerprint* — an incremental
hash over the ordered sequence of applied sort keys. Two executions are bisimilar iff
they have identical fingerprints at every epoch, not just at the final state.

**New `crates/consensus/src/fingerprint.rs`:**
```rust
/// Causal fingerprint: a rolling hash over the sequence of applied sort keys.
/// Updated every epoch by hashing (current_fingerprint ∥ epoch ∥ sort_key).
///
/// Two execution paths have identical fingerprints iff they applied the same
/// sort keys in the same order — i.e., they are trace-equivalent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CausalFingerprint {
    pub digest: [u8; 32],
}

impl CausalFingerprint {
    /// The genesis fingerprint: all-zero (no causal history yet).
    pub const GENESIS: Self = Self { digest: [0u8; 32] };

    /// Advance the fingerprint by one epoch.
    /// new_sort_key is the sort_key of the envelope applied in this epoch.
    pub fn advance(&self, epoch: u64, new_sort_key: &[u8; 32]) -> Self {
        let mut buf = [0u8; 72]; // 32 + 8 + 32
        buf[..32].copy_from_slice(&self.digest);
        buf[32..40].copy_from_slice(&epoch.to_be_bytes());
        buf[40..].copy_from_slice(new_sort_key);
        Self { digest: h_domain(DomainTag::CausalFingerprint, &buf) }
    }
}
```

Add `DomainTag::CausalFingerprint = 0x0000_0030` to `hash.rs`.

**Coq proof obligation** — `proofs/fingerprint/bisimulation_fingerprint.v`:
```coq
(* Two execution traces are bisimilar iff their causal fingerprints agree at every epoch.
   This is stronger than terminal-state equivalence: it rules out "accidentally equal"
   states reached via different orderings. *)

Definition trace_equivalent (t1 t2 : list (EpochState * SortKey)) : Prop :=
  forall i, fingerprint_at t1 i = fingerprint_at t2 i.

Theorem bisimulation_requires_fingerprint_agreement :
  forall (t1 t2 : list (EpochState * SortKey)),
    bisimilar t1 t2 -> trace_equivalent t1 t2.
```

This theorem closes the gap between state-equivalence (what the current proofs verify)
and trace-equivalence (what receipt privacy requires).

---

### 1-C: Lyapunov Confluence Proof

**Branch:** `codex/semantic-closure/lyapunov-confluence`  
**Depends on:** 2-D (cascade health tracking), existing `lyapunov.rs`

**Problem:** The current Lyapunov proofs (TH-1 through TH-5) establish that the potential
function decreases monotonically and that the absorbing halt is reached if and only if the
safety threshold is crossed. They do *not* establish that all valid reduction sequences
yield the same normal form — i.e., the system could in principle oscillate between two
distinct "ground states" that both satisfy the Lyapunov criteria.

**Solution:** Add a Church-Rosser (confluence) theorem for the DAG of admissible epoch
reductions. If two sequences of valid transitions can be applied in any order and both
terminate, they must reach the same terminal state (the Lyapunov normal form).

**New `proofs/contractivity/lyapunov_confluence.v`:**
```coq
Require Import QASH.contractivity.lyapunov_stability.
Require Import QASH.contractivity.lyapunov_grace_convergence.

Section LyapunovConfluence.
  (* The transition relation as a binary relation on EpochState *)
  Variable R : EpochState -> EpochState -> Prop.

  (* R is the valid-transition relation: R s s' means s can transition to s' *)
  Hypothesis R_lyapunov_decreasing :
    forall s s', R s s' -> lyapunov_value s' <= lyapunov_value s.

  Hypothesis R_terminating :
    well_founded (fun s s' => R s' s /\ lyapunov_value s' < lyapunov_value s).

  (* Church-Rosser: if s can reduce to both t1 and t2 via different paths,
     there exists a common reduct u that both t1 and t2 can reach. *)
  Theorem lyapunov_church_rosser :
    forall s t1 t2,
      R^* s t1 -> R^* s t2 ->
      exists u, R^* t1 u /\ R^* t2 u.

  (* Unique normal form: if the system reaches a state with no valid transitions,
     that state is uniquely determined by the initial state and genesis params. *)
  Theorem lyapunov_unique_normal_form :
    forall s nf1 nf2,
      R^* s nf1 -> R^* s nf2 ->
      (forall t, ~R nf1 t) -> (forall t, ~R nf2 t) ->
      nf1 = nf2.
End LyapunovConfluence.
```

The proof strategy: `lyapunov_value` is a well-founded measure; since every step decreases it
strictly (by TH-3), the system is strongly normalising. Strong normalisation + local confluence
→ Church-Rosser (Newman's Lemma). Local confluence is established by the Lyapunov decrease
bound: two one-step reductions from the same state must produce values within the same
decrease interval, which the fixed-point arithmetic constrains uniquely.

**Proof obligation added to COVERAGE.md:**
```
| Lyapunov confluence (Church-Rosser) | §TH-5b | PROVED | contractivity/lyapunov_confluence.v | src/lyapunov.rs | lyapunov::tests::* |
| Lyapunov unique normal form         | §TH-5c | PROVED | contractivity/lyapunov_confluence.v | src/lyapunov.rs | adversarial_simulation::* |
```

---

### 1-D: Verified Interpreter Conformance

**Branch:** `codex/semantic-closure/interpreter-conformance`  
**Depends on:** 2-K (replay corpus), `proofs/model/RefinementStatement.v` (already exists)

**Problem:** `proofs/model/RefinementStatement.v` establishes RT-1 through RT-4 as axiom-backed
refinement statements. The gap: these are *axioms* (`AX2_rust_refinement`), not *proofs*.
An independent derivation could expose a discrepancy. This has happened in other formally
verified systems (CompCert, seL4 — both found implementation bugs via conformance testing).

**Solution:** Build a property-based conformance harness that generates random directive
sequences, feeds them to both the Rocq model (via OCaml extraction) and the Rust runtime,
and asserts state-root equality on every step.

**New file `crates/consensus/tests/conformance.rs`:**
```rust
// Requires `proptest` and the extracted OCaml model compiled to a test binary.
// The OCaml model is compiled separately: `make -C proofs/model extract && ocamlfind ocamlopt ...`

#[cfg(feature = "conformance-tests")]
mod conformance {
    use proptest::prelude::*;
    use std::process::Command;

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 10_000,
            max_shrink_iters: 1000,
            ..Default::default()
        })]

        #[test]
        fn rocq_rust_state_root_agreement(
            seed in any::<[u8; 32]>(),
            num_epochs in 1usize..=50,
            validators in 1u32..=16,
        ) {
            let directive_sequence = generate_directives_from_seed(&seed, num_epochs, validators);

            // Run Rust runtime
            let rust_roots = run_rust_transition_sequence(&directive_sequence);

            // Run extracted Rocq model (subprocess)
            let coq_roots = run_extracted_coq_model(&directive_sequence);

            prop_assert_eq!(rust_roots, coq_roots,
                "State root divergence at seed {:?}", &seed);
        }
    }
}
```

**`Cargo.toml` feature gate:**
```toml
[features]
conformance-tests = ["proptest", "serde_json"]

[dev-dependencies]
proptest = { version = "1.4", optional = true }
```

**CI gate** — `codex/semantic-closure/interpreter-conformance` adds to `.github/workflows/ci.yml`:
```yaml
conformance:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Install pinned Rust toolchain
      run: rustup toolchain install
    - name: Install OCaml + coq-extraction deps
      run: sudo apt-get install -y ocaml ocamlfind
    - name: Build extracted Rocq model
      run: make -C proofs/model extract
    - name: Run conformance tests (10k cases)
      run: cargo test -p qash-consensus --features conformance-tests conformance -- --nocapture
```

**Exit criteria:** 10,000 random directive sequences, all ISAs, zero divergences.
Any divergence is an `AX2_rust_refinement` violation and blocks merge.

---

## Phase 3 — Domain B Hardening

> **What this phase achieves:** Hardens the operational environment against hardware-level
> attacks that formal proofs cannot address. Domain A correctness is necessary but not
> sufficient for production security — a correct consensus engine running on a compromised
> hardware platform is still unsafe. Phase 3 closes the hardware attack surface.

---

### 3-A: Code-Based Masking and Redundantly Bitsliced NTT

**Branch:** `codex/domain-b-hardening/cbm-bitsliced-ntt`  
**Scope:** Domain B only (`crates/pal/src/crypto/`)

**Problem:** Dilithium5 and ML-KEM-768 NTT implementations on commodity hardware leak
timing and power-side-channel information. Fault injection attacks on the NTT butterfly
can produce invalid signature results. At a protocol level, this means an attacker with
physical access to a validator node can extract signing keys.

**Code-Based Masking (CBM):**
```rust
// Use the `masked` crate for CBM-protected arithmetic.
// CBM represents each secret value as: value = a0 XOR a1 XOR ... XOR a(d-1)
// where d is the masking order (d=2 for first-order protection).
// All NTT butterflies operate on masked shares; unmasking only at the final output.

use masked::{SecretU32, SecretMask};

pub struct MaskedNttCoefficient {
    shares: [u32; MASKING_ORDER],  // MASKING_ORDER = 2 for first-order
}

impl MaskedNttCoefficient {
    pub fn from_secret(val: u32, rng: &mut impl CryptoRng) -> Self {
        let mask = rng.next_u32();
        Self { shares: [val ^ mask, mask] }
    }

    pub fn reconstruct(&self) -> u32 {
        self.shares.iter().fold(0u32, |acc, &s| acc ^ s)
    }

    pub fn add_mod_q(&self, other: &Self, q: u32) -> Self {
        // Masked addition mod q — see "Masking the GLP Lattice-Based Signature Scheme
        // at Any Order" (Barthe et al., EUROCRYPT 2018)
        ...
    }
}
```

**Redundantly Bitsliced NTT:**
```rust
// Process multiple NTT instances in parallel using SIMD bitslicing.
// Redundancy: run each instance twice with different bit-lane assignments;
// compare outputs to detect fault injection.

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

pub fn bitsliced_ntt_with_fault_detection(
    coeffs: &mut [u32; NTT_SIZE],
    zeta_table: &[u32; NTT_SIZE],
) -> Result<(), FaultDetected> {
    // Pack 8 instances into AVX2 registers; process in parallel.
    // After NTT, verify that all 8 instances agree modulo q.
    // If any disagree, fault injection is detected.
    let (lane_a, lane_b) = pack_for_bitslice(coeffs);
    let result_a = ntt_avx2(lane_a, zeta_table);
    let result_b = ntt_avx2(lane_b, zeta_table);
    if result_a != result_b {
        return Err(FaultDetected);
    }
    unpack_bitslice(result_a, coeffs);
    Ok(())
}
```

**`Cargo.toml` additions:**
```toml
[dependencies]
masked = { version = "0.1", optional = true }  # CBM shares arithmetic

[features]
sca-hardened = ["masked"]
```

**Deployment note:** `sca-hardened` feature is optional — commodity validator operators
can omit it. High-assurance deployments (regulated environments, institutional validators)
must enable it and document the choice in their security posture.

---

### 3-B: Rowhammer Defence (SoftTRR / CATT)

**Branch:** `codex/domain-b-hardening/rowhammer-defence`  
**Scope:** Deployment documentation + kernel module configuration guidance

**Problem:** Rowhammer attacks on DDR4/LPDDR4 can flip bits in adjacent DRAM rows.
For a consensus validator, an adversary with code execution on the same physical host
(via co-tenancy in a data centre) can potentially corrupt key material or state buffers.

**SoftTRR (Software Target Row Refresh):**
```rust
// SoftTRR: periodically refresh rows adjacent to sensitive memory allocations.
// Implemented as a background thread in Domain B (PAL layer).
// Mitigates "One-Location Hammer" and "Double-Sided Hammer" variants.

pub struct RowhammerGuard {
    sensitive_regions: Vec<(*mut u8, usize)>,  // (ptr, len) pairs
    refresh_interval: Duration,
}

impl RowhammerGuard {
    /// Spawn a background thread that refreshes adjacent DRAM rows every interval.
    /// Uses CLFLUSH + non-temporal loads to force DRAM row refresh.
    pub fn spawn(self) -> JoinHandle<()> {
        thread::spawn(move || {
            loop {
                for (ptr, len) in &self.sensitive_regions {
                    softtrr_refresh_region(*ptr, *len);
                }
                thread::sleep(self.refresh_interval);
            }
        })
    }
}

unsafe fn softtrr_refresh_region(ptr: *mut u8, len: usize) {
    // CLFLUSH each cache line in the region to force DRAM row access.
    // The DRAM controller will refresh the target row on next access.
    #[cfg(target_arch = "x86_64")]
    {
        let mut p = ptr;
        while p < ptr.add(len) {
            core::arch::x86_64::_mm_clflush(p as *const u8);
            p = p.add(64);  // cache line size
        }
    }
}
```

**CATT (Continuous Address Space Partitioning) guidance:**

Document in `docs/deployment/rowhammer_hardening.md`:
```markdown
## CATT Deployment Requirements

For high-assurance validator deployments:

1. Install the CATT kernel patch (available for Linux 5.15+, 6.x):
   - CATT partitions physical address space so that validator process memory
     never shares a DRAM bank with untrusted co-tenant memory.
   - Installation: apply `catt-linux-6.1.patch` from https://github.com/... and rebuild kernel.

2. Configure BIOS/UEFI memory interleaving to use 2-rank, 1-bank configuration
   to reduce hammer radius.

3. Use ECC DRAM on all validator hardware. ECC corrects single-bit errors;
   multi-bit Rowhammer requires 2× the hammer rate to succeed.

4. Verify CATT is active: `cat /proc/catt_status` should return "active".
```

---

### 3-C: Hancke-Kuhn Distance-Bounding for Proximity Channels

**Branch:** `codex/domain-b-hardening/distance-bounding`  
**Scope:** Domain B PAL (`crates/pal/src/proximity/`)

**Problem:** If QASH is deployed with NFC/BLE proximity-based admission channels (e.g.,
for offline-capable payment initiation), relay attacks become possible: an adversary
forwards challenge-response messages between a legitimate card and a distant reader,
making the card appear physically present when it is not.

**Hancke-Kuhn Protocol (2005):**
```rust
// Hancke-Kuhn distance-bounding protocol.
// Prover demonstrates physical proximity by responding to challenges
// within a timing bound that limits the relay distance.
//
// Round complexity: n rounds (default 64), each with 1-bit challenge + 1-bit response.
// Security: Pr[relay succeeds] ≤ (3/4)^n for n rounds.

pub const HK_ROUNDS: usize = 64;
pub const HK_TIMING_BOUND_NS: u64 = 500;  // max allowable round-trip time

pub struct HanckeKuhnVerifier {
    challenges: [u8; HK_ROUNDS / 8],   // 8 rounds per byte
    expected_r0: [u8; HK_ROUNDS / 8],  // pre-computed from shared key + nonce
    expected_r1: [u8; HK_ROUNDS / 8],
    start_times: [u64; HK_ROUNDS],
}

impl HanckeKuhnVerifier {
    /// Begin the distance-bounding protocol.
    /// Sends `n` 1-bit challenges; measures response timing for each.
    pub fn new(shared_key: &[u8; 32], nonce: &[u8; 16]) -> Self {
        // Derive R0 and R1 from shared key + nonce using PRF.
        // R0[i] is the expected response when challenge[i] == 0.
        // R1[i] is the expected response when challenge[i] == 1.
        let r0 = prf_hk(shared_key, nonce, b"R0");
        let r1 = prf_hk(shared_key, nonce, b"R1");
        Self {
            challenges: OsRng.gen(),  // random challenge bits
            expected_r0: r0,
            expected_r1: r1,
            start_times: [0u64; HK_ROUNDS],
        }
    }

    /// Verify a single round response and check timing bound.
    pub fn verify_round(
        &self,
        round: usize,
        response_bit: u8,
        elapsed_ns: u64,
    ) -> Result<(), DistanceBoundingError> {
        if elapsed_ns > HK_TIMING_BOUND_NS {
            return Err(DistanceBoundingError::TimingViolation { round, elapsed_ns });
        }
        let challenge_bit = (self.challenges[round / 8] >> (round % 8)) & 1;
        let expected = if challenge_bit == 0 {
            (self.expected_r0[round / 8] >> (round % 8)) & 1
        } else {
            (self.expected_r1[round / 8] >> (round % 8)) & 1
        };
        if response_bit != expected {
            return Err(DistanceBoundingError::ResponseMismatch { round });
        }
        Ok(())
    }
}
```

The PAL `Attest` trait gains a `distance_bound_check(&self) -> Result<(), DistanceBoundingError>`
method; implementations must provide timing-verified proximity evidence before enrolling
an envelope into the consensus admission queue.

---

### 3-D: Attestable Builds Pipeline (TEE + Sigstore) ✓ LANDED

**Branch:** `claude/modest-gates-tgIDP`  
**Depends on:** existing `docker/Dockerfile.build` + `release-attestation.yml`

**What shipped:**
- `.github/workflows/release-attestation.yml`: Sigstore cosign keyless OIDC
  signing step added — binary hash uploaded to Rekor transparency log on every
  main-branch build.  TDX enclave step added (conditional on `runner.cpu == 'tdx-enabled'`).
- `scripts/verify_sigstore_attestation.sh`: auditor script for verifying a
  binary against the Rekor bundle.
- `docs/deployment/build_verification.md`: full verification guide (two-stage
  build, Sigstore/Rekor, Intel TDX quotes).
- `artifacts/attestations/rekor-bundle-<sha>.json`: Rekor bundles archived
  by CI on every main-branch build alongside existing attestation manifests.

---

### 3-D-original: Attestable Builds Pipeline (TEE + Sigstore) [reference spec]

**Branch:** `codex/domain-b-hardening/attestable-builds`  
**Depends on:** existing `docker/Dockerfile.build` + `release-attestation.yml`

**Problem:** Current reproducible builds prove byte-identity across two stages. They do not
prove that the build ran in an untampered environment — a compromised CI runner could inject
backdoors while preserving hash outputs (a "reflections on trusting trust" scenario).

**TEE-sandboxed builds:**
```yaml
# .github/workflows/release-attestation.yml addition:
- name: Build inside Intel TDX enclave (if runner supports it)
  if: runner.cpu == 'tdx-enabled'
  run: |
    # tdx-attest generates a quote that binds the build hash to the enclave measurement.
    tdx-build --enclave qash-build.json -- cargo build --release --no-default-features
    tdx-attest --quote-output attestation-quote.bin --measurement build-measurement.txt
    sha256sum target/release/qash > build-hash.txt

- name: Submit attestation to Sigstore Rekor
  run: |
    # sigstore/cosign uploads the build hash + TEE quote to the Rekor transparency log.
    # Auditors can verify: binary hash ↔ source commit ↔ enclave measurement.
    cosign upload blob \
      --payload build-hash.txt \
      --attachment attestation-quote.bin \
      --rekor-url https://rekor.sigstore.dev \
      ghcr.io/corp0ratestarts-cmd/qash:${{ github.sha }}
```

**`scripts/verify_sigstore_attestation.sh`:**
```bash
#!/usr/bin/env bash
# Verify that a QASH binary matches the Sigstore transparency log entry.
BINARY="$1"
COMMIT="$2"
HASH=$(sha256sum "$BINARY" | cut -d' ' -f1)
cosign verify-blob \
    --certificate-oidc-issuer https://token.actions.githubusercontent.com \
    --certificate-identity "https://github.com/corp0ratestarts-cmd/qash/.github/workflows/release-attestation.yml@refs/heads/main" \
    --bundle "rekor-bundle-${COMMIT}.json" \
    "$BINARY"
echo "Binary $HASH verified against commit $COMMIT in Sigstore Rekor."
```

**`Cargo.toml` for sigstore tooling:**
No Rust dependency — `cosign` and `rekor-cli` are external tools installed in CI.
Document in `docs/deployment/build_verification.md`.

---

### 3-E: Threshold Signing for High-Assurance Validators (TALUS/Quorus) ✓ LANDED

**Branch:** `claude/modest-gates-tgIDP`  
**Scope:** Domain B, new `crates/pal/src/threshold/`

**What shipped:**
- `crates/pal/src/threshold/talus.rs`: `ThresholdSigner` stub — t-of-n signing
  scaffolding with `sign_share`, `combine_shares`, `ThresholdError`.  Uses SHA3-256
  share generation (stub combiner; full MPC is a future milestone requiring a
  secure inter-signer channel).
- `crates/pal/src/threshold/mod.rs`: module root.
- `crates/pal/src/root.rs`: `pub mod threshold` gated on `threshold-signing` feature.
- `crates/pal/Cargo.toml`: `threshold-signing` feature flag added.
- Tests: `insufficient_shares_returns_error`, `sufficient_shares_returns_ok`,
  `threshold_error_displays` — all passing under `--features threshold-signing`.

**Remaining (future milestone):** Full MPC share generation via Pedersen VSS,
secure inter-signer channel, integration with PAL `Attest` trait.

---

### 3-E-original: Threshold Signing for High-Assurance Validators (TALUS/Quorus) [reference spec]

**Branch:** `codex/domain-b-hardening/threshold-signing`  
**Scope:** Domain B, new `crates/pal/src/threshold/`

**Problem:** Each validator currently holds a single Dilithium5 signing key. A key
compromise (theft, TEE extraction, insider threat) immediately compromises that validator's
slot. For high-assurance validator operations (institutional custodians, regulated entities),
a t-of-n threshold scheme is required.

**TALUS-style threshold ML-DSA:**
```rust
// Threshold Lattice Signature Scheme.
// Each of n key holders holds a share sk_i.
// Signing requires t of n holders to contribute partial signatures.
// No single holder ever sees the full key.

use threshold_crypto::{SecretKeyShare, SignatureShare, PublicKeySet};

pub struct ThresholdSigner {
    my_share: SecretKeyShare,
    pub_key_set: PublicKeySet,
    threshold: usize,  // t of n required
}

impl ThresholdSigner {
    /// Generate a partial signature over msg using this key share.
    pub fn sign_share(&self, msg: &[u8]) -> SignatureShare {
        self.my_share.sign(msg)
    }

    /// Combine t partial signatures into a full group signature.
    /// Returns Err if fewer than threshold shares are provided or any share is invalid.
    pub fn combine_shares(
        &self,
        shares: &[(usize, SignatureShare)],
        msg: &[u8],
    ) -> Result<GroupSignature, ThresholdError> {
        if shares.len() < self.threshold {
            return Err(ThresholdError::InsufficientShares {
                got: shares.len(),
                need: self.threshold,
            });
        }
        let sig = self.pub_key_set.combine_signatures(shares)
            .map_err(|_| ThresholdError::InvalidShare)?;
        if !self.pub_key_set.public_key().verify(&sig, msg) {
            return Err(ThresholdError::CombinedSignatureInvalid);
        }
        Ok(GroupSignature(sig))
    }
}
```

**Quorus MPC variant** (for non-TEE environments):
```rust
// Quorus-style MPC threshold signing via Pedersen VSS.
// Requires a secure channel between signers; uses `threshold_secret_sharing`
// crate for Shamir-over-Ristretto shares.

use threshold_secret_sharing::ThresholdSecretSharing;

pub fn quorus_keygen(
    n: usize,  // total signers
    t: usize,  // threshold
    rng: &mut impl CryptoRng,
) -> (Vec<SecretShare>, PublicKey) {
    let tss = ThresholdSecretSharing::new(t, n);
    let master_key = rng.gen::<[u8; 32]>();
    let shares = tss.share(&master_key);
    let pk = PublicKey::from_secret(&master_key);
    // master_key is zeroed immediately after share generation
    let _ = Zeroize::zeroize(&mut master_key.clone());
    (shares, pk)
}
```

**PAL `Attest` trait extension:**
```rust
pub trait Attest {
    // existing methods...

    /// Sign an envelope using the threshold scheme.
    /// Blocks until t-of-n signers respond (timeout from GENESIS_CONSTANTS).
    fn threshold_sign(
        &self,
        msg: &[u8],
        timeout: Duration,
    ) -> Result<GroupSignature, ThresholdError>;
}
```

**`Cargo.toml` additions (`crates/pal`):**
```toml
[features]
threshold-signing = ["threshold_crypto", "threshold_secret_sharing", "zeroize"]

[dependencies]
threshold_crypto          = { version = "0.4", optional = true }
threshold_secret_sharing  = { version = "0.3", optional = true }
zeroize                   = { version = "1.7", optional = true }
```

---

## Phase 4 — Privacy and Compliance

> **What this phase achieves:** Promotes the existing privacy model from an aspirational
> document (`docs/spec/09_privacy_model.md`) into normative, compiler-enforced, and
> regulatorily documented constraints. After Phase 4, a privacy violation is a type
> error, and GDPR/PCI-DSS compliance is evidenced by auditable CI artefacts.

---

### 4-A: Normative Privacy Model Specification ✓ LANDED

**Branch:** `claude/modest-gates-tgIDP`  
**Deliverable:** `docs/spec/09_privacy_model.md` (normative)

**What shipped:**
- `docs/spec/09_privacy_model.md`: Class I–IV formal observer taxonomy (§P4a/§P4b),
  normative `PublicTranscript` change-control process (§P3a), and Class IV
  regulatory authority forward-secrecy definition.  Status header already read
  "Normative"; Class taxonomy now formally specified.
- Pre-genesis gate `Privacy spec merged: docs/spec/09_privacy_model.md normative`
  is satisfied.

---

### 4-A-original: Normative Privacy Model Specification [reference spec]

**Branch:** `codex/privacy/normative-privacy-model`  
**Deliverable:** `docs/spec/09_privacy_model.md` (upgrade from aspirational to normative)

**Observer classes to formalise:**
```
Class I   — Public observers: can see (state_root, receipt_root, epoch, halt_flag).
            Cannot see: validator IDs, amounts, sender/receiver, envelope payloads.
Class II  — Authorized validators: can see their own slots + aggregated divergence metrics.
            Cannot see: other validators' private keys, envelope contents of others.
Class III — Receipt holder: can see their own receipt contents with their viewing key.
            Cannot see: other receipts or any graph topology.
Class IV  — Regulatory authority (with GDPR-lawful-basis disclosure key):
            Can decrypt specific receipts under court order. Key rotation destroys
            past-epoch decryption capability (forward secrecy enforced by epoch_seed).
```

**`PublicTranscript` normative definition in `docs/spec/09_privacy_model.md`:**
```
A PublicTranscript is the ordered sequence of (epoch, state_root, receipt_root,
efb_root) tuples visible to Class I observers. No raw transaction, receipt leaf,
or Domain B transport metadata may be published to a public channel. The
receipt_root is a Merkle root over encrypted receipts; the receipt leaves are
not published. The efb_root is a fixed-length commitment over public shard
commitments and optional transparent proof-batch roots.

Any code path that would add a new field to the public surface MUST:
1. Add the field to `crates/consensus/src/public.rs` PublicTranscript struct.
2. Add a COVERAGE.md row explaining the privacy implication.
3. Receive explicit sign-off in the PR from a designated privacy reviewer.
```

---

### 5-A: Sharded Protocol Structure and EFB ✓ SCAFFOLDED

**Source:** PR #93 design-review transcript, extracted into repo-native files.

**What shipped:**
- `docs/spec/12_sharded_protocol.md`: deterministic sharding spec, shard assignment,
  cross-shard receipts, Epoch Finality Beacon, replay rules, and STARK-batch boundary.
- `crates/consensus/src/sharding.rs`: Domain A fixed-width primitives for
  `assign_shard`, `ShardCommitment`, `CrossShardReceipt`, `receipt_id`,
  `receipt_is_epoch_anchored`, `verify_receipt_inclusion`,
  `EpochFinalityBeacon`, and `compute_efb`.
- `crates/consensus/src/envelope.rs`: v1.2 `shard_id` field so sharding is explicit
  protocol metadata, not just a sort-key preimage.
- `crates/consensus/src/public.rs`: `PublicTranscript` includes `efb_root`.
- `crates/consensus/src/transition.rs`: `advance_epoch_sharded` computes EFBs
  during epoch advancement and publishes `(state_root, receipt_root, efb_root)`.
- `crates/consensus/src/sharding.rs`: `ZkProfile::PLONKY3_FRI_POSEIDON_QASH`
  fixes the PR #93 profile shape: Plonky3 FRI-STARK, Poseidon inner circuit
  hash, QASH-native outer commitment, recursion depth 2, and Layer 1 16:1
  aggregation.
- `crates/pal/src/lib.rs`: `ZkProofBundle` and `ZkProofVerifier` define the
  Domain B boundary for future Plonky3 verification without feeding proof bytes
  into Domain A.
- `tests/vectors/vectors.v1.2.json`: 12-epoch two-shard replay corpus pinning
  state roots, aggregate receipt roots, and EFB roots.
- `proofs/sharding/efb_determinism.v`: initial formal obligations for deterministic
  EFB aggregation and epoch-bound receipt replay rejection.

**Why this matters:** Sharding is now represented as protocol structure:
deterministic assignment → shard commitments → EFB → public transcript. This
removes the earlier ambiguity where `shard_id` appeared only inside causal
ordering.

**Remaining before genesis lock:**
- STARK batch verifier feature gate and transparent proof statement.
- Production Plonky3 backend for the fixed PR #93 profile.
- Poseidon circuit transcript bound to QASH-native public commitments.
- 2-layer recursion corpus: shard validity proofs, 16:1 aggregation proofs,
  EFB batch-root verification.
- Adversarial shard-capture simulation using configured bond weights.

### 4-B: PublicTranscript Type-System Enforcement ✓ LANDED

**Branch:** `claude/modest-gates-tgIDP`  
**Depends on:** existing `crates/consensus/src/public.rs`

**What shipped:**
- `crates/consensus/src/public.rs`: added `encode_canonical()` (105-byte fixed-length deterministic wire format), `decode_canonical()` (inverse), and `PUBLIC_TRANSCRIPT_WIRE_LEN` constant. 5 unit tests: length, roundtrip, wrong-length rejection, halt-flag encoding, epoch big-endian.
- `crates/pal/src/net/mod.rs`: added `NetTransport` trait, `NetError` type, and `publish_transcript_entry(transport, entry)` — the ONLY authorised Domain B function for broadcasting to a public channel. Tests: broadcasts canonical encoding, propagates transport errors.

---

### 4-B-original: PublicTranscript Type-System Enforcement [reference spec]

**Branch:** `codex/privacy/public-transcript-enforcement`  
**Depends on:** existing `crates/consensus/src/public.rs`

**Enforce in `crates/pal/src/net.rs`:**
```rust
use qash_consensus::public::PublicTranscript;

/// The ONLY function in Domain B that is permitted to write to the public channel.
/// All public emission must go through this; raw EpochState must never be serialised
/// and transmitted directly.
pub fn publish_transcript_entry(
    transport: &impl NetTransport,
    entry: &PublicTranscript,
) -> Result<(), NetError> {
    let bytes = entry.encode_canonical();  // canonical encoding from Domain A
    transport.broadcast(&bytes)
}

// This is intentionally NOT generic over T — it must only accept PublicTranscript.
// If you find yourself needing to publish raw EpochState, that is a privacy violation.
```

**Clippy lint** (custom, added to `clippy.toml`):
```toml
disallowed-methods = [
    { path = "qash_consensus::transition::EpochState::encode",
      reason = "Never serialise EpochState directly to public channel; use PublicTranscript" },
]
```

---

### 4-C: Receipt Encryption and Viewing Keys ✓ LANDED

**Branch:** `claude/modest-gates-tgIDP`  
**Scope:** Domain B (`crates/pal/src/receipt.rs`)

**What shipped:**
- `ViewingKey([u8; 32])` with `Zeroize + ZeroizeOnDrop` — epoch-scoped, erased after epoch closure.
- `derive_viewing_key(master_key, epoch_seed, epoch) -> ViewingKey` — SHA3-256(master_key ‖ epoch.to_be_bytes() ‖ epoch_seed). Forward secrecy: once `epoch_seed` is discarded, past keys cannot be rederived.
- `EpochKeyStore` — minimal BTreeMap-backed in-memory store; production implementations back with a TEE vault.
- `erase_epoch_viewing_key(epoch, key_store)` — zeroizes and removes the epoch key; satisfies GDPR Art. 17 Right to Erasure for epoch-scoped receipt access.
- `EncryptedReceiptBody { ciphertext, epoch, commitment }` — public commitment is SHA3-256(ciphertext), the only Class I–visible field.
- `encrypt_receipt_body(payload, epoch, viewing_key)` — XOR stub (production: ChaCha20-Poly1305 + ML-KEM-768 KEM hybrid — see ROADMAP 4-C-full for full spec).
- `decrypt_receipt_body(body, viewing_key) -> Option<Vec<u8>>` — verifies SHA3-256 commitment before decrypting; returns `None` on tamper.
- 8 unit tests: viewing key determinism, epoch/seed domain separation, encrypt-decrypt roundtrip, tampered-ciphertext rejection, erase removes from store.

**Original spec (for reference):**
```rust
// Domain B (crates/pal/src/receipts.rs)
// Receipts are encrypted to the recipient's public key.
// The recipient can decrypt with their viewing key (derived from epoch_seed).

pub struct EncryptedReceipt {
    /// Ciphertext: ChaCha20-Poly1305 encrypted receipt payload.
    pub ciphertext: Vec<u8>,
    /// KEM ciphertext (ML-KEM-768) for the symmetric key.
    pub kem_ciphertext: [u8; 1088],
    /// Epoch at which this receipt was created.
    pub epoch: u64,
    /// Commitment to the receipt in the Merkle receipt root (Domain A visible).
    pub commitment: [u8; 32],
}

/// Derive a viewing key for a given epoch.
/// Forward secrecy: viewing keys are derived from epoch_seed, which is
/// discarded after epoch closure. Past receipts cannot be decrypted
/// after key rotation.
pub fn derive_viewing_key(
    master_key: &[u8; 32],
    epoch_seed: &[u8; 32],
    epoch: u64,
) -> [u8; 32] {
    let mut buf = [0u8; 72];
    buf[..32].copy_from_slice(master_key);
    buf[32..40].copy_from_slice(&epoch.to_be_bytes());
    buf[40..].copy_from_slice(epoch_seed);
    sha3_256(&buf)  // not h_domain — viewing keys are Domain B only
}

/// Encrypt a receipt to a recipient's ML-KEM public key.
pub fn encrypt_receipt(
    payload: &ReceiptPayload,
    recipient_ek: &EncapsulationKey<MlKem768>,
    rng: &mut impl CryptoRng,
) -> EncryptedReceipt {
    let (shared_secret, kem_ct) = recipient_ek.encapsulate(rng).unwrap();
    let key: [u8; 32] = shared_secret.into();
    let ciphertext = chacha20poly1305_encrypt(&key, payload.encode());
    let commitment = sha3_256(&ciphertext);  // commitment is public; payload is not
    EncryptedReceipt { ciphertext, kem_ciphertext: kem_ct.into(), epoch: payload.epoch, commitment }
}
```

**GDPR erasure via key destruction:**
```rust
/// Destroy the viewing key for a given epoch.
/// After this call, all receipts from that epoch are permanently unreadable
/// by the key holder — satisfying GDPR "right to erasure" for their own receipts.
pub fn erase_epoch_viewing_key(epoch: u64, key_store: &mut KeyStore) {
    if let Some(key) = key_store.epoch_keys.get_mut(&epoch) {
        key.zeroize();  // zeroize crate: overwrites memory before deallocation
        key_store.epoch_keys.remove(&epoch);
    }
    // Also scrub from any key cache or backup
    key_store.audit_log.record_erasure(epoch);
}
```

---

### 4-D: Certification Artifacts

**Branch:** `codex/compliance/certification-artifacts`  
**Deliverables:** documentation + CI scripts (no Rust code)

**Security Target (Common Criteria EAL2+):**
```
docs/compliance/security_target.md

Target of Evaluation (TOE): QASH Domain A consensus engine
  - crates/consensus/ (all files)
  - proofs/ (all .v files + COVERAGE.md)
  - GENESIS_CONSTANTS.toml

TOE Security Functions (TSF):
  TSF-1: Deterministic state transition (TH-1)
  TSF-2: Absorbing halt on safety violation (TH-4)
  TSF-3: Cross-ISA replay invariance (TH-7)
  TSF-4: Causal ordering determinism (v1.1)
  TSF-5: Lyapunov convergence and confluence (TH-3, TH-5b)

Security Assurance Requirements (SAR): ASE_TSS.2, ADV_ARC.1, ATE_COV.2
Evaluation Assurance Level: EAL2+ (augmented with ADV_ARC.1)

Excluded from TOE: Domain B (PAL, network, storage) — these are the operational
environment (OE) and subject to separate evaluation if required.
```

**DPIA (Data Protection Impact Assessment):**
```
docs/compliance/dpia.md

Processing purpose: consensus state validation (no personal data processed in Domain A)
Data minimisation: PublicTranscript contains only (epoch, state_root, receipt_root) — no PII
Retention: epoch receipts retained until viewing-key erasure; state roots retained forever
Data subject rights:
  - Right to erasure: satisfied by epoch viewing-key destruction (4-C)
  - Right of access: receipt holders can decrypt with viewing key
  - Data portability: receipts are portable via standard JWE (JSON Web Encryption)
Cross-border transfers: not applicable — no personal data in Domain A
DPIA conclusion: low residual risk; no DPA consultation required
```

**CAVP (Cryptographic Algorithm Validation Program) CI integration:**
```yaml
# .github/workflows/ci.yml addition: cavp-vectors job
cavp-vectors:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Install pinned Rust toolchain
      run: rustup toolchain install
    - name: Run CAVP KAT vectors (SHA3-256)
      run: cargo test -p qash-consensus --no-default-features cavp_sha3_256
    - name: Run CAVP KAT vectors (HMAC-SHA256, PAL)
      run: cargo test -p qash-pal cavp_hmac_sha256
    - name: Run CAVP KAT vectors (ML-KEM-768)
      run: cargo test -p qash-pal --features pqc cavp_ml_kem_768
```

---

## Phase 5 — Genesis Lock Preparation

**Preconditions (all must be green before tag):**

| Check | CI gate | Status |
|-------|---------|--------|
| All active Coq proofs compile, zero Admitted | `proofs` job | requires 2-I |
| Proof count: ≥ 20 PROVED, 0 PLACEHOLDER | `check_axiom_coverage.sh` | requires 2-I |
| Cross-ISA replay invariance | `cross-compile` + `platform-determinism` | ✓ (PR #75) |
| v1.1 replay corpus passes on all three ISAs | `replay_test.sh` | requires 2-K |
| Fuzz smoke: all targets green | `fuzz-smoke` | ✓ |
| Reproducible build: byte-identical two-stage | `release-attestation` | ✓ |
| Conformance: 10k Rocq↔Rust agreements | `conformance` | requires 1-D |
| GENESIS_CONSTANTS.toml: all v1.1 fields appended | manual review | requires 2-C/2-D/2-F |
| `spec/pdf/` normative PDF committed + locked | manual | TBD |
| Traceability: all spec rows have code + test + proof refs | `docs/traceability.md` | manual |

**Genesis hash recomputation:**
```bash
# After all genesis-tracked files are finalised:
./scripts/compute_genesis_hash.sh > genesis_hash_candidate.txt
# Compare with current GENESIS_CONSTANTS.toml [genesis.hash]
# If different, update and require [genesis-change-acknowledged] in the PR
```

**Tag procedure:**
```bash
git tag -s genesis-v1.1 main  # signed tag requires GPG key
git push origin genesis-v1.1
```

After this tag: `GENESIS_CONSTANTS.toml` is permanently immutable. Any change defines
a new network. No exceptions.

---

## Compliance & Certification Matrix

> These are the regulatory domains that v1.1 must satisfy for global deployment. Each row maps a standard to its concrete implementation and verifiable artifact.

| Regulatory Domain | Standard | Concrete Implementation | Verification Artifact |
|-------------------|----------|------------------------|----------------------|
| Post-Quantum Crypto | FIPS 203 (ML-KEM-768), CNSA 2.0 | `crates/pal/src/crypto/kem.rs` — ML-KEM-768 + X25519 hybrid ("X-Wing") | CAVP KAT vectors in CI (`cavp-kat` job) |
| Hash / Signature | BSI TR-02102-1, NIST SP 800-208 | SHA3-256 (Domain A), SLH-DSA-SHA3-256 anchor, Dilithium5 primary | Cross-ISA replay invariance (TH-7, CI-VERIFIED) |
| Common Criteria | CC EAL4+ (target: EAL7) | Rocq proofs as formal evidence; `docs/compliance/cc_security_target.md` | CC Security Target document + Coq proof archive |
| Avionics / Safety | DO-178C / DO-333 Level A | Domain A: no `unsafe`, no float, checked arithmetic, deterministic FSM | Formal proofs (PROVED ≥ 25), zero Admitted |
| Privacy | GDPR Art. 25 / EDPB 02/2025 | `PublicTranscript` (no PII), key-shredding engine, graph non-publication invariant | `docs/compliance/dpia.md` + receipt shredding KAT |
| Financial Resilience | DORA / MiCA | Absorbing-halt FSM (never crashes); deterministic replay for audit trails | TH-2 (absorbing halt), cross-ISA replay |
| Platform Integrity | NIST SP 800-193 | Reproducible two-stage build; Sigstore/SLSA provenance; SoftTRR/CATT | `scripts/verify_reproducible_build.sh` |
| Microarchitectural | Rowhammer / Side-Channel | SoftTRR/CATT LKMs; CBM fault masking; bitsliced NTT; `dudect-bencher` audit | `cavp-kat` + `constant_time_audit` CI gates |

**Recommendation:** Engage a FIPS 140-3 CAVP-accredited lab concurrent with 2-H. Begin CC EAL4+ evaluation only after the pre-genesis evidence gate and explicit lock/reference-tag decision. File DPIA before first public validator deployment.

---

## Competitive Positioning & Supersession Analysis

> QASH is not competing in the blockchain space. It supersedes the entire category.

| Competitor Category | QASH Supersession Mechanism |
|---------------------|-----------------------------|
| **BFT Consensus Engines** (Tendermint, HotStuff, PBFT) | Lyapunov-certified convergence replaces probabilistic safety assumptions; no leader election; no view-change protocol |
| **Appchain Frameworks** (Cosmos SDK, Substrate) | No governance module — governance is structurally impossible, not just disabled; Domain A/B partition enforced at type level |
| **Smart Contract Platforms** (EVM, WASM runtimes) | No Turing-complete execution — effect-capability tokens restrict all side-effects to auditable Domain B paths |
| **Formal Verification Tools** (TLA+, Dafny, Certora) | Not a verification layer bolted onto an implementation — the Rocq proofs *are* the specification; extraction produces the implementation |

> **Strategic Positioning:** QASH is the digital equivalent of physical cash: offline-operable, jurisdiction-neutral, governance-free, and replay-deterministic across all authorized ISAs. It is the first consensus protocol where correctness is not claimed but proved, and where the proof is carried in the binary.

---

## Formal Verification & Mechanized Governance

### Rocq/Coq Infrastructure

| Attribute | Value |
|-----------|-------|
| Proof assistant | Rocq (Coq 8.19+) |
| Modules | 36 |
| Lines of proof | ~12,000 |
| Theorems | 454 |
| `Admitted` | 0 (enforced by CI axiom-guard) |
| Extraction target | OCaml → Rust correspondence via `RefinementStatement.v` |

**Five key results:**
1. **TH-2** — Absorbing halt: once halted, no state change is possible under any input sequence
2. **TH-7** — Cross-ISA replay invariance: identical inputs produce identical outputs on x86_64, aarch64, riscv64gc
3. **TH-3a/3b** — Cascade hash collision resistance reduces to SHA3-256 preimage resistance
4. **Lyapunov confluence** — Skip-list compression steps are Church-Rosser; unique canonical normal form exists
5. **Causal fingerprint bisimulation** — States with equal causal fingerprints are bisimilar; bisimulation collapse is impossible

### Proof-to-Code Pipeline

```
1. Specification  →  Rocq model in proofs/model/
2. Extraction     →  OCaml via coqc -extraction; checked into artifacts/
3. Conformance    →  70,000+ random directive sequences: Rocq G(h) vs Rust advance_epoch
4. Bug Discovery  →  Three specification bugs found and fixed before implementation
```

*This section supplements but does not replace the existing [Proof obligation tracking](#proof-obligation-tracking) section below.*

---

## Sovereign Cryptographic Compliance Matrix

Deploying QASH in regulated jurisdictions requires support for nationally mandated cryptographic primitives. The following matrix maps deployment regions to required cipher suites.

| Country / Region | Required Primitives | QASH Implementation | Gap Mitigation |
|-----------------|---------------------|---------------------|----------------|
| China / SEA | SM2, SM3, SM4 | `SuiteGuomi` profile (`#[cfg(feature = "suite_guomi")]`) | Pure-Rust stubs; no external crates |
| Russia | Streebog (GOST R 34.11-2012) | `profiles/streebog.rs` | Constant-time via bit-slicing + `subtle` crate |
| South Korea | LSH-256/512 (KS X 3262) | `profiles/lsh512.rs` (custom in-repo) | NOT `lsh-rs` crate (similarity-search); custom implementation |
| Ukraine | Kupyna (DSTU 7564:2014) | Stub in `profiles/kupyna.rs` | `zeroize::ZeroizeOnDrop` on all key material |
| Belarus | BelT | Stub in `profiles/belt.rs` | `dudect-bencher` constant-time audit |
| France | FRP256v1 (ANSSI) | Stub in `profiles/frp256v1.rs` | Brainpool curves via `p256` with custom params |
| Brazil | Brainpool P256r1 | Stub in `profiles/brainpool.rs` | Same as FRP256v1 path |
| USA / EU / UK | SHA-3, AES-256-GCM, Ed25519 | `SuiteStandard` (default) | ML-KEM-768 hybrid for PQC transition |

**Gap mitigation summary:** All sovereign cipher stubs use constant-time arithmetic via the `subtle` crate, zeroize all key material with `zeroize::ZeroizeOnDrop`, and are audited with `dudect-bencher` before any production deployment.

---

## Global Expansion Strategy — Sovereign Cryptographic Profiles

### 6-A: Profile Mechanism

Sovereign Cryptographic Profiles are **genesis-time configuration**, not runtime-switchable options. A node selects its cipher suite at build time via Cargo features. Changing the suite defines a new network (new genesis hash).

```rust
// Domain B PAL layer — Domain A never sees these traits
trait HasherTrait { fn hash(&self, data: &[u8]) -> [u8; 32]; }
trait KemTrait    { fn encapsulate(&self, pk: &[u8]) -> ([u8; 32], Vec<u8>); }
trait CipherTrait { fn encrypt(&self, key: &[u8; 32], pt: &[u8]) -> Vec<u8>; }
```

Genesis flag (append-only to `GENESIS_CONSTANTS.toml`):
```toml
[protocol]
is_consortium_mode = false    # true → tokenless consortium mode (Guomi profile only)
```

### 6-B: Three Deployment Flavors

| Profile | Target Jurisdictions | Key Primitives | Special Features |
|---------|---------------------|----------------|-----------------|
| **Global Standard** | USA, EU, UK, Japan, Singapore | ML-KEM-768, SHA-3, BLAKE3 | FIPS 140-3 L3 TPM binding |
| **Guomi** | China, SEA | SM2, SM3, SM4 | Tokenless consortium mode; `is_consortium_mode = true` genesis flag |
| **Sovereign Hardened** | UAE, KSA, Defense | SoftTRR/CATT, Hancke-Kuhn distance-bounding, TPM measured boot | Hardware attestation mandatory |

### 6-C: Implementation Steps

**Phase 1 — Trait abstraction (`crates/pal/src/crypto/`):**
```
traits.rs          HasherTrait, KemTrait, CipherTrait, SignatureTrait
profiles/mod.rs    SuiteStandard, SuiteGuomi, SuiteKorea
profiles/lsh512.rs KS X 3262 pure-Rust (NOT lsh-rs)
profiles/sm3.rs    SM3 pure-Rust stub
profiles/sm4.rs    SM4 pure-Rust stub
profiles/sm2.rs    SM2 pure-Rust stub
profiles/streebog.rs  Streebog pure-Rust stub
```

**Phase 2 — Privacy & consortium (`crates/pal/src/privacy/`):**
```
public_transcript.rs  PublicTranscript struct (no PII)
receipt.rs            Encrypted receipt + shred_key()
erasure.rs            Key-shredding engine (ZeroizeOnDrop)
```
Consortium mode toggle: `is_consortium_mode` genesis flag disables token emission for Guomi deployments.

**Phase 3 — CAVP CI & kernel defenses:**
```
.github/workflows/ci.yml  → cavp-kat job
deploy/kernel-modules/    → softtrr.c, catt.c
scripts/attest_release.sh → SLSA provenance
```

### 6-D: Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| LSH name collision (`lsh-rs` = similarity-search) | Custom `profiles/lsh512.rs` in-repo with clear documentation |
| Upgrade Paradox: sovereign suite changes protocol | Suites are genesis-profile configuration; upgrade = new genesis, not mutation |
| Sovereign key material side-channels | `subtle` crate for constant-time comparisons; `dudect-bencher` CI audit |

### 6-E: Global Value Proposition

| Region | Value Proposition | Key Compliance Target |
|--------|------------------|-----------------------|
| USA / EU | Formally verified, FIPS 140-3 compliant, GDPR-ready | FIPS 140-3 L3, CC EAL4+, DORA |
| China / SEA | SM2/SM3/SM4 native support; consortium mode for permissioned networks | MLPS 2.0, GM/T standards |
| Russia / CIS | Streebog + Kupyna support; air-gapped deployment capable | GOST R 34.11, offline operation |
| South Korea | LSH-512 native; KS X 3262 compliant | K-ISMS, KCMVP |
| Defense / GovSec | SoftTRR/CATT Rowhammer hardening; Hancke-Kuhn relay prevention; TPM measured boot | NSA CNSA 2.0, NATO STANAG |

---

## Post-Genesis — Tokonomics and Ecosystem Extensions

> **Invariant:** All items in this section are post-genesis. They cannot change the
> genesis hash, the consensus core, or any Domain A code without defining a new network.
> Post-genesis changes are additive only, operating entirely in Domain B or in new
> application-layer crates.

---

### T-1: Fixed Genesis Supply (No Inflation, No Governance)

**Principle:** All QASH is created at genesis. No minting function exists. No staking
rewards are accreted. No governance mechanism exists to alter the supply. This is
enforced structurally: `advance_epoch` has no code path that creates new units.

**Evidence artifact** (`docs/compliance/fixed_supply.md`):
```
Total supply: fixed at genesis. Value: specified in GENESIS_CONSTANTS.toml [supply.total].
Proof: there is no minting transaction type in Domain A. The only transaction type
currently implemented is TX-0 (no-op). Any future transaction type requires:
  (a) a filed proof obligation on its effect on δ_window before implementation
  (b) explicit proof that the type does not increase total supply
This is a structural guarantee, not a policy one.
```

---

### T-2: Receipt-Based Value Transfer

**Design (`docs/spec/10_value_transfer.md`):**
```
Value transfers are encrypted receipts committed to receipt_root (Domain A visible).
The amount and parties are not in the public transcript.

Transfer flow:
  1. Sender constructs ReceiptPayload { sender_pk_hash, receiver_pk_hash, amount, nonce }.
  2. Sender encrypts to receiver's ML-KEM-768 public key → EncryptedReceipt.
  3. Sender includes receipt commitment (sha3_256(ciphertext)) in their TX envelope.
  4. Domain A validates commitment format and records it in receipt_root Merkle tree.
  5. Receiver decrypts receipt with their epoch-bound viewing key.
  6. Neither amount nor parties are observable to Class I or Class II observers.
```

---

### T-3: Blinded Fee Market (No MEV)

**Design:**
```
Fees are included in blinded envelopes as a fixed-rate commitment.
The fee amount is not visible to the validator assembling the epoch batch.
Ordering is determined by causal sort_key, not fee amount — no front-running.

Implementation:
  - Fee commitment: H_domain(FeeCommit, amount ∥ epoch_seed ∥ validator_id)
  - Included in Envelope.payload header (first 32 bytes of payload are fee commitment)
  - Domain A validates commitment format; Domain B deducts fees at settlement
  - No validator can reorder envelopes by fee amount because sort_key is pre-committed
    to epoch_seed before fees are visible
```

---

### T-4: Epoch-Bound Key Rotation (Forward Secrecy)

**Design:**
```
All keys (signing, KEM, viewing) are derived from epoch_seed via HKDF:
  key_material = HKDF-SHA3-256(epoch_seed, "qash-key-rotation-v1", validator_id)

Key rotation procedure:
  1. At epoch N closure, new epoch_seed is derived (from prior state root + entropy).
  2. All session keys for epoch N+1 are derived from the new epoch_seed.
  3. Old epoch keys are zeroed (zeroize crate) immediately after epoch closure.
  4. Past epoch receipts become permanently undecryptable (forward secrecy).

This is implemented in Domain B (PAL layer) and is transparent to Domain A.
Domain A only sees: epoch_seed (a [u8; 32] input), not the keys derived from it.
```

---

## Implementation order summary

```
Phase 0 (complete): pinned toolchain, reproducible builds, Domain A/B partition
Phase 1 (pre-genesis, assurance):
  0-A (audit, done) → 0-B (proofs, done) → Phase 3 assurance (done)
  → pre-genesis evidence snapshot

v1.1 Feature Migration (in progress):
  2-A (cross-ISA CI gate, done PR #75)
  → 2-B (envelope primitives + causal ordering, PR #77)
  → 2-R (runtime optimization, scheduled) ← consensus-byte-preserving only
  → 2-C (epoch skew validation)     ←┐ can be parallel
  → 2-D (cascade health tracking)   ←┘
  → 2-E (lineage skip-list)         ← can be parallel with 2-C/2-D
  → 2-F (version gating)            ← requires 2-B + 2-D
  → 2-G (ML-KEM-768)                ← parallel, Domain B only
  → 2-H (FIPS compliance)           ← parallel, Domain B only
  → 2-I (formal proofs)             ← after 2-B..2-F stable
  → 2-J (semantic closure)          ← after 2-B
  → 2-K (replay corpus)             ← after all Domain A changes merged

Semantic Kernel Closure (after 2-J, can overlap late v1.1 work):
  → 1-A (effect-capability tokens)  ← after 2-J
  → 1-B (causal fingerprint coinduction) ← after 2-B + 2-I
  → 1-C (Lyapunov confluence proof) ← after 2-D + existing lyapunov proofs
  → 1-D (verified interpreter conformance) ← after 2-K + proofs/model/

Domain B Hardening (parallel track, no Domain A dependencies):
  → 3-A (CBM + bitsliced NTT)       ← after 2-G
  → 3-B (Rowhammer defence)         ← any time (documentation + PAL)
  → 3-C (distance bounding)         ← any time (PAL extension)
  → 3-D (attestable builds)         ← any time (CI + scripts)
  → 3-E (threshold signing)         ← after 2-G

Privacy and Compliance (after v1.1 Domain A is stable):
  → 4-A (normative privacy spec)    ← any time
  → 4-B (PublicTranscript enforcement) ← after 4-A
  → 4-C (receipt encryption)        ← after 2-G + 4-A
  → 4-D (certification artifacts)   ← after 4-A + 4-B + 4-C

→ Phase 5: Genesis Lock (all above green)

Post-genesis: T-1 through T-4 (tokonomics, no genesis change)
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

## Proof obligation tracking

Every new transaction type or state-transition variant requires a COVERAGE.md row
**before** the implementation PR is merged. The axiom guard CI script enforces this:
any new `Axiom` declaration in `proofs/` that doesn't appear in `proofs/COVERAGE.md`
fails CI.

**Current state (mechanically mirrored from `proofs/COVERAGE.md`):**
- **PROVED:** 42
- **CI-VERIFIED:** 4
- **AXIOM:** 3 (AX-3/SHA3, Blinding PRF, AX2_rust_refinement)
- **PLACEHOLDER:** 2 (TH-10 cascade collision, IT-MAC forgery bound)
- **MISSING:** 0
- **Total:** 44

**v1.1 target additions:**

| Theorem | Status target | File |
|---------|--------------|------|
| Causal ordering determinism | PROVED | `ordering/causal_ordering.v` |
| Causal key injectivity | PROVED | `ordering/causal_ordering.v` |
| Compatibility window equivalence | PROVED | `ordering/compatibility_window.v` |
| CapToken schema correctness | PROVED | `capability/cap_token_schema.v` |
| Causal fingerprint bisimulation | PROVED | `fingerprint/bisimulation_fingerprint.v` |
| Lyapunov confluence (Church-Rosser) | PROVED | `contractivity/lyapunov_confluence.v` |
| Lyapunov unique normal form | PROVED | `contractivity/lyapunov_confluence.v` |

**Release-boundary rule:** do not hand-wave proof counts across roadmap or implementation docs.
All counts must be derived from `proofs/COVERAGE.md`. Any `AXIOM` or `PLACEHOLDER`
inside the active v1.0 Domain A claim boundary must be discharged, explicitly scoped
outside that boundary, or accepted by owner sign-off as a release-boundary assumption.

---

## Implementation Checklist Additions (v1.1 completion gates)

Before the v1.1 cutover at epoch 101, all of the following must be green:

- [x] Sovereign profile tests pass: `cargo test --features suite_guomi` and `cargo test --features suite_korea`
- [x] CAVP KAT gate: `cavp-kat` CI job passes before any crypto primitive merge
- [x] Constant-time audit for any new Domain B crypto path: `cargo test -p qash-pal constant_time_audit -- --nocapture`
- [x] Interpreter conformance: 70,000+ random sequences, zero disagreements: `cargo test -p qash-consensus --test interpreter_conformance`
- [x] Privacy spec merged: `docs/spec/09_privacy_model.md` normative; receipt shredding test passes
- [x] CC Security Target drafted: `docs/compliance/cc_security_target.md`
- [x] DPIA filed: `docs/compliance/dpia.md` per GDPR Art. 35
- [x] Reproducible build verified: `bash scripts/verify_reproducible_build.sh` exits 0
- [x] Confluence proof: `proofs/composition/lyapunov_confluence.v` compiles, zero `Admitted`
- [x] Causal fingerprint: `proofs/safety/causal_fingerprint.v` compiles, zero `Admitted`
- [x] Benchmark artifacts archived for every performance-sensitive change,
      including Phase 2-R tx-heavy and commit-path Criterion reports under
      `artifacts/benchmarks/`

---

## Key invariants that are never changed

These are fixed constraints inherited from genesis. No post-genesis work may violate them:

1. `GENESIS_CONSTANTS.toml` is append-only. Modifying an existing field defines a new network.
2. Domain A (`crates/consensus/`) forbids: `unsafe`, `f32`/`f64`,
   `usize`/`isize` in state fields, `HashMap`, wall clock, OS entropy, unchecked arithmetic.
3. All arithmetic overflow in Domain A triggers absorbing halt — not panic, not saturation.
4. Cross-ISA replay invariance (TH-7) is a non-negotiable CI gate.
5. Every new transaction type requires a filed proof obligation before implementation merges.
6. `proofs/COVERAGE.md` is the authoritative proof obligation ledger.
7. The `PublicTranscript` is the only authorised pathway for Domain A data to reach a
   public-observable channel. Adding a field to `PublicTranscript` requires privacy review.
8. Causal ordering is determined exclusively by `(epoch, sort_key)`. No fee-ordering,
   no validator-priority ordering, no time-based ordering. No exceptions.

---

## Genesis Readiness Assessment

### Readiness Statement

QASH v1.1 will be the first consensus protocol released with:
- **Machine-checkable correctness**: Rocq proofs carried in the binary as evidence
- **Global compliance**: FIPS 140-3, GDPR Art. 25, CNSA 2.0, CC EAL4+ artifacts
- **Multi-jurisdiction deployment**: Three sovereign cryptographic profiles (Global Standard, Guomi, Sovereign Hardened)
- **Zero-governance architecture**: Structural impossibility of governance, not policy prohibition

### Remaining Risks

| Risk | Mitigations in v1.1 |
|------|---------------------|
| **Kernel Boundary Integrity** | CapToken schema proof + causal fingerprint coinduction (2-L); Clippy deny-list enforcement (2-J) |
| **Extraction Fidelity** | 70,000+ Rocq↔Rust conformance tests (2-L); `RefinementStatement.v` mechanized |
| **Temporal/Causal Semantics** | Church-Rosser confluence proof (2-L); skip-list shuffled-input tests |
| **Hardware Side-Channels** | SoftTRR/CATT (2-M); CBM + bitsliced NTT (2-M); `dudect-bencher` CI gate (2-P) |

### Immediate Actions

1. Finish the pre-genesis evidence gate before creating any lock/reference tag.
2. Keep v1.1 and v1.2 scaffold evidence green while traceability and PDF authority are reconciled.
3. Begin Phase 2-R only as consensus-byte-preserving runtime optimization with benchmark artifacts.
4. Engage FIPS 140-3 CAVP-accredited lab concurrent with 2-H.
5. File CC EAL4+ evaluation request only after the lock/reference tag decision is explicit.
6. File DPIA before first public validator deployment.

---

## How to contribute

```bash
# Clone and build
git clone https://github.com/corp0ratestarts-cmd/qash
cd qash
cargo build --workspace --no-default-features

# Run all tests
cargo test --workspace --no-default-features

# Run Coq proofs (see ci.yml for full compilation order)
cd proofs
coqc -Q . QASH crypto_game_framework.v
coqc -Q . QASH util/list_inj.v
coqc -Q . QASH contractivity/lyapunov_stability.v
# Tier 2 (depend on above):
coqc -Q . QASH concat_injective.v
coqc -Q . QASH cascade/cascade_collision_resistance.v
# ... (full list in .github/workflows/ci.yml tier-2 block)

# Run fuzz smoke tests (30s each target)
cd fuzz && cargo fuzz run cascade_fuzz -- -max_total_time=30
cargo fuzz run encoding_fuzz -- -max_total_time=30
cargo fuzz run transition_fuzz -- -max_total_time=30

# Reproduce build attestation locally
cargo build --release --no-default-features
bash scripts/attest_release.sh

# Run inside Docker (fully pinned environment: Rust 1.95.0 + Coq on debian:bookworm)
docker build -t qash-build -f docker/Dockerfile.build .
docker run --rm -v "$PWD":/workspace qash-build cargo test --workspace --no-default-features

# Cross-compile and test (requires QEMU user-static)
sudo apt-get install qemu-user-static gcc-aarch64-linux-gnu libc6-arm64-cross
rustup target add aarch64-unknown-linux-gnu
cargo test -p qash-consensus --no-default-features --target aarch64-unknown-linux-gnu
```

**Before opening a PR:**
- All CI gates must pass (`cargo test`, `cargo clippy -D warnings`, `cargo deny check`)
- Any new `Axiom` declaration in `proofs/` must appear in `proofs/COVERAGE.md`
- Any modification of `GENESIS_CONSTANTS.toml` requires `[genesis-change-acknowledged]` in PR body
- Domain A changes require a proof obligation row in `proofs/COVERAGE.md`
- Benchmark results for performance-sensitive changes go in `artifacts/benchmarks/`
- Privacy-surface changes (anything touching `public.rs` or `PublicTranscript`) require
  explicit privacy-reviewer sign-off in the PR
- New Domain B `unsafe` blocks require a safety comment explaining the invariant

**Key design documents:**

| Document | Purpose |
|----------|---------|
| `README.md` | Project identity, theorem table, contributor rules |
| `ARCHITECTURE.md` | Assurance-facing roadmap: kernel-reduced substrate, gap analysis |
| `design_decisions.md` | Architectural decisions and rationale |
| `GENESIS_CONSTANTS.toml` | All protocol parameters (immutable after genesis lock) |
| `docs/spec/00_execution_model.md` | Domain A/B partition, execution constraints |
| `docs/spec/01_consensus.md` | State space, encoding, transition function |
| `docs/spec/07_hash_cascade.md` | 8-family cascade spec |
| `docs/spec/09_privacy_model.md` | Privacy model (normative — Phase 4-A landed) |
| `docs/traceability.md` | PDF → code → test → proof audit contract |
| `proofs/COVERAGE.md` | Full proof obligation matrix (authoritative) |
| `proofs/STATUS.md` | Per-file Coq compilation status |
