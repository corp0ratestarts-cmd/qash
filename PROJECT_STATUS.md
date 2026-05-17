# QASH — Project Status and Strategic Roadmap

> **Audience:** Technical reviewers, external auditors, prospective contributors, and investors.
> This document gives an honest current-state assessment, maps known gaps to specific actions,
> and establishes a prioritized roadmap toward genesis lock and production readiness.
>
> **Last updated:** May 2026. Reflects the independent audit of the architecture and
> consensus implementation by an external reviewer.

---

## What QASH Is (One Paragraph)

QASH is a deterministic replicated transition calculus — a replicated state machine whose
primary invariant is *identical replay produces identical state* across all authorized ISAs.
It is post-quantum anchored (8-family hash cascade, ML-DSA/SLH-DSA signing), formally
machine-verified for its core safety properties (14 proved Coq theorems, zero `Admitted`
markers), and designed for zero governance: once `GENESIS_CONSTANTS.toml` is locked, no
protocol upgrade is possible without a new network. The architecture is closer to avionics
software than to a conventional blockchain node.

---

## Honest Current State

### What is solid

| Area | Assessment |
|------|-----------|
| Determinism discipline | **Excellent.** Domain A/B partition is rigorously enforced. No `unsafe`, no floats, no nondeterministic collections, no wall-clock access in consensus. |
| Formal safety proofs | **Strong.** 14 theorems proved in Coq with zero `Admitted`. Covers encoding injectivity, Lyapunov convergence, halt correctness, succession soundness, TX-0/TX-1 perturbation bounds. |
| Cross-ISA replay verification | **CI-verified.** Identical state roots on x86_64, aarch64, riscv64gc (QEMU) on every PR. |
| Serialization discipline | **Correct.** Manual little-endian encoding, fixed-width slots, strict decode validation (rejects malformed padding, invalid halt codes, nonzero reserved fields). |
| Fuzz coverage | **Gate in place.** Three Domain A fuzz targets (cascade, decode, transition) run 10k iterations per PR via honggfuzz on stable Rust. |
| Proof coverage map | **Complete.** Every stated protocol property has an explicit status in `proofs/COVERAGE.md`. No silent gaps. |

### What is not yet solid

| Gap | Risk level | Detail |
|-----|-----------|--------|
| Runtime operational maturity | **High** | PAL Host returns zeroes/no-ops. Network, persistence, crash recovery, and distributed operation are not implemented. This is not a deployable node. |
| Reproducible build pipeline | **Medium** | No Nix/Docker pinned environment, no byte-identical release attestations, no multi-compiler differential testing. AX-2 (compiler correctness) is trusted without tooling. |
| Proof CI artifact trail | **Medium** | Coq proofs compile but there are no machine-readable CI proof hashes, no extraction reproducibility docs, no Coq ↔ Rust refinement proofs. External auditors cannot independently verify proof reproducibility without running Coq locally. |
| Adversarial halt simulation | **Medium-High** | The absorbing-halt architecture is correct by construction, but the attack surface for liveness suppression (malformed inputs triggering halt, economic griefing via validator slot manipulation) has not been adversarially exercised. |
| Fixed-size array scaling | **Medium** | `MAX_VALIDATORS = 1024` with large fixed arrays creates worst-case stack/memory footprint concerns. No benchmark exists for worst-case epoch transition cost. |
| Open proof obligations | **Low-Medium** | 1 PLACEHOLDER (TH-10: cascade collision resistance requires hash model in EasyCrypt/CryptHOL), 3 AXIOMs (blinding PRF, IT-MAC forgery bound, SHA3 collision resistance). All are mathematically justified but not mechanised. |

---

## Strategic Priorities

### Priority 1 — Proof reproducibility pipeline (before genesis lock)

The audit's highest-confidence finding: proof claims currently exceed the visible audit trail.

**Actions:**
- Add Coq proof hash check to `ci.yml` — compute `sha256sum proofs/**/*.v` at compile time and store in `proofs/PROOF_HASHES.txt`, verified on CI.
- Publish a pinned Coq Docker image (`coq:8.18` + `mathcomp`) so any reviewer can re-verify locally with a single `docker run` command.
- Document the extraction pipeline: how Coq `Model.v` maps to `model/` Rust, and what invariants are checked at the boundary.
- Add a `verify-proofs.sh` script at the repo root that re-runs `coqc` on all non-`_wip` files and reports zero failures.

**Files affected:** `ci.yml`, `proofs/PROOF_HASHES.txt` (new), `scripts/verify-proofs.sh` (new), `proofs/README.md` (new).

---

### Priority 2 — Adversarial halt simulation (before genesis lock)

The absorbing halt design is the right choice for a deterministic system. Its attack surface
must be adversarially exercised before any network carries real value.

**Actions:**
- Extend `transition_fuzz` to cover specifically the halt-triggering boundary: values near
  `ε_halt` (20k), `SLASH_MAX`, `WEIGHT_BH`-triggered cascades, and validator count edge cases.
- Write a dedicated `halt_grief_sim.rs` test that simulates an economically adversarial validator
  submitting minimal divergence variations across epochs to find the cheapest halt-trigger path.
- Verify that the 10× tolerance margin (ε_honest=2k, ε_halt=20k) holds under all `FuzzInput`
  combinations — not just the current `within_epsilon_does_not_halt` golden test.

**Files affected:** `fuzz/fuzz_targets/transition_fuzz.rs`, `crates/consensus/tests/halt_grief_sim.rs` (new).

---

### Priority 3 — Deep audit of four core modules

The external auditor identified four modules requiring close inspection before any genesis lock
claim can be independently validated.

**`fixed_point.rs`**
- Verify that all checked arithmetic paths genuinely reach `absorbing_reset()` on overflow — no
  silent saturation or truncation.
- Document the rounding invariant (truncating division vs. floor) and test it at boundary values.
- Add a proptest that generates random `FixedPoint` pairs and verifies `a * b` never silently
  exceeds `i128::MAX / SCALE`.

**`encoding.rs`**
- Verify canonicalization: no two distinct `EpochState` values produce the same byte sequence
  (TH-1 is proved in Coq; this needs an independent Rust-level injectivity test with proptest).
- Audit all `decode_*` paths for malformed-input rejection completeness.
- Add a corpus of known-bad byte sequences (truncated, misaligned, wrong version) as regression vectors.

**`lyapunov.rs`**
- Stress-test monotonicity: `V_convergence` must never increase across an epoch under any valid
  input. The Coq proof covers the model; the Rust implementation should have a proptest mirroring it.
- Verify adversarial convergence: a validator set designed to maximize `δ_window` while staying
  below `ε_halt` should not be able to prevent eventual halt indefinitely.

**`cascade.rs` + `hash.rs`**
- Verify domain separation: `h_cascade(x)` and `h_cascade_keyed(k, x)` must not collide for
  any fixed `k ≠ ∅` (probabilistic; fuzz already covers this partially).
- Audit constant-time behavior: none of the 8 hash primitives must branch on secret inputs.
  This is a property of the underlying crates (sha3, blake3, etc.) — document the guarantee
  and add a CI note referencing each crate's constant-time claims.

**Files affected:** `crates/consensus/src/fixed_point.rs`, `crates/consensus/tests/fixed_point_props.rs` (new), `crates/consensus/tests/encoding_props.rs` (new), `crates/consensus/tests/lyapunov_props.rs` (new).

---

### Priority 4 — Reproducible builds and binary transparency

AX-2 (compiler correctness) is a load-bearing axiom. The current repo has no tooling to
validate it independently.

**Actions:**
- Add `rust-toolchain.toml` at the root pinning the exact stable channel + component set
  (already partially done in some sub-crates; needs to be canonical and enforced).
- Add a `Dockerfile.build` that produces a byte-identical binary from a clean Ubuntu image.
- Add a GitHub Actions workflow `reproducible-build.yml` that builds twice in parallel, hashes
  both binaries, and fails if they differ.
- Publish a `BUILD_ATTESTATION.md` template for release artifacts: toolchain hash, target triple,
  build flags, binary SHA3-256.

**Files affected:** `rust-toolchain.toml` (root), `Dockerfile.build` (new), `.github/workflows/reproducible-build.yml` (new), `docs/BUILD_ATTESTATION.md` (new).

---

### Priority 5 — PAL host implementation (operational maturity)

The runtime is currently a CLI demo. This is honest and documented, but it means the protocol's
operational properties — network partitioning behavior, crash recovery, synchronization — are
entirely untested.

This is the largest gap between design quality and deployment readiness.

**Milestone: Testnet-capable node**

Required PAL implementations (in order of dependency):
1. `Time` trait — monotonic clock, epoch boundary detection
2. `Net` trait — peer discovery, message broadcast, receive loop
3. Persistence — canonical `EpochState` write-ahead log
4. `Attest` trait — hardware attestation stub (TPM or software fallback)
5. `Halt` trait — clean shutdown + state preservation on absorbing halt

None of these require Domain A changes. They are Domain B (`crates/pal/`) work with `unsafe`
permitted under audit.

**Files affected:** `crates/pal/src/hosted/` (new implementations), `src/main.rs` (wire PAL traits).

---

### Priority 6 — Discharge remaining open proof obligations

| Obligation | Path to discharge | Effort |
|-----------|------------------|--------|
| TH-10: Cascade collision resistance | Model SHA3/BLAKE3 in EasyCrypt or CryptHOL; reduce to AX-3. Alternatively, axiomatise in Coq with a documented computational assumption (lower effort, lower assurance). | High |
| Blinding PRF (AX-cascade_prf) | Formal proof in CryptHOL or SSProve that `H_cascade_keyed` is a PRF under AX-3. | High |
| IT-MAC forgery bound | Mechanise GF(2¹²⁸) GHASH reduction in Coq. | Medium |

Until TH-10 is discharged, the cascade collision resistance claim rests on AX-3 (SHA3 collision
resistance) by reduction — mathematically sound but not mechanically verified.

---

## Constraints That Will Not Change

These are design properties, not limitations. Any proposed change that violates them requires
a new network, not a protocol upgrade.

1. **No floating point in Domain A** — ever.
2. **No unsafe in Domain A** — ever.
3. **`GENESIS_CONSTANTS.toml` is append-only** — modification defines a new network.
4. **No governance, no upgrade mechanism** — the protocol is one-shot by design.
5. **Replay invariance is the primary invariant** — all other properties are subordinate to it.
6. **Overflow → absorbing halt** — never saturation, never silent truncation.
7. **BTreeMap, not HashMap** — deterministic iteration order is non-negotiable.

---

## Milestone Summary

| Milestone | Key deliverables | Target |
|-----------|-----------------|--------|
| **M1: Audit-ready** | Proof CI hashes, verify-proofs.sh, adversarial halt simulation, fixed_point/encoding/lyapunov property tests | Pre-genesis lock |
| **M2: Genesis lock** | All theorems proved (no PLACEHOLDER), GENESIS_CONSTANTS.toml locked, reproducible build attestation | Genesis event |
| **M3: Testnet** | PAL host implementation (Time, Net, persistence, Halt), single-node operation, basic peer sync | Post-genesis |
| **M4: Adversarial testnet** | Multi-validator adversarial simulation, economic griefing resistance validated, halt-recovery documented | Pre-mainnet |
| **M5: Mainnet** | Binary transparency, independent external audit sign-off, deployment documentation | Mainnet launch |

---

## What This Repo Is Not Yet

To be explicit for any reader evaluating this project:

- **Not a deployable node.** The PAL host returns zeroes/no-ops. No network, no persistence.
- **Not independently audited.** The proofs are machine-checked by Coq but have not been
  reviewed by an external formal-methods team.
- **Not benchmarked.** Worst-case epoch transition cost, serialization throughput, and replay
  latency are not yet measured.
- **Not production-hardened.** Crash recovery, adversarial network behavior, and validator
  slot exhaustion attacks have not been exercised.

The architecture is unusually rigorous for a project at this stage. The gap is execution
maturity, not conceptual soundness.

---

*This document is updated alongside significant architectural or milestone changes.*
*It does not replace the normative spec PDF or `docs/traceability.md`.*
