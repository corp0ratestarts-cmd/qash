# QASH — Project Status and Strategic Roadmap

> **Audience:** External auditors, potential contributors, and technical reviewers.
> This document gives an honest account of what has been built, what the open gaps
> are, and what the priority order for closing them is. It is updated as milestones
> are reached.
>
> Last updated: 2026-05-19. Based on internal review, the current CI workflow,
> and an independent external audit of the architecture, workspace, and consensus
> implementation.

---

## What QASH Is (One Paragraph)

QASH is a deterministic replicated transition calculus — a formally constrained
state machine whose correctness properties are intended to be machine-checkable
from explicit axioms. The design goal is the digital equivalent of physical cash:
offline-operable, jurisdiction-neutral, governance-free, and replay-deterministic
across all authorized ISAs. The central invariant is: **identical replay produces
identical state**, with no exceptions in Domain A. This is achieved through strict
language-level and architectural constraints on the consensus core (`crates/consensus`),
enforced at the type level and by CI.

For full context see `README.md`, `design_decisions.md`, and `docs/spec/00_execution_model.md`.

---

## Honest Current State

| Dimension | Status | Notes |
|-----------|--------|-------|
| Consensus core correctness | **Strong** | `no_std`, `forbid(unsafe_code)`, determinism constraints fully enforced |
| Formal proof coverage | **Strong** | 18 PROVED, 4 CI-VERIFIED, 3 AXIOM, 2 PLACEHOLDER — see `proofs/COVERAGE.md` |
| Proof CI pipeline | **Automated for active Coq proofs** | `.github/workflows/ci.yml` installs Coq, rejects active `Admitted`/`admit` markers, checks new axioms against `proofs/COVERAGE.md`, compiles active `.v` files, records `.vo` SHA-256 hashes, and uploads version/hash artifacts |
| Runtime / PAL implementation | **Scaffold only** | PAL Host returns zeroes/no-ops; no network, no persistence, no crash recovery |
| Fuzzing infrastructure | **Expanded** | honggfuzz harness covers encoding, decode, transition, fixed-point, lyapunov, cascade, and tx targets; fuzz-smoke CI gate runs all targets |
| Performance benchmarks | **None** | Worst-case epoch transition cost, serialization throughput, stack depth: unmeasured |
| Reproducible builds | **Done** | `rust-toolchain.toml` pins 1.95.0; `docker/Dockerfile.build` pins full build+proof environment; `release-attestation.yml` CI job verifies byte-identical two-stage builds and records SHA-256 manifests under `artifacts/attestations/` with 365-day retention |
| Adversarial simulation | **Done** | 23-test suite across 10 scenarios: halt-trigger boundary, liveness suppression, coordinated spike, nonce replay, max-field saturation, slash monotonicity, halt irreversibility, grace period |
| Deep module audits | **Done** | `fixed_point.rs`, `encoding.rs`, `lyapunov.rs`, `hash.rs`, `transaction.rs` audited and hardened with boundary/adversarial tests (PRs #69, #71) |
| Multi-compiler differential | **Done** | opt-level=0 vs opt-level=3 required gate + cranelift advisory; weekly scheduled CI (PR #65) |
| Production readiness | **Pre-production** | Intentionally — this is a protocol design and formal proof repository |

---

## What the External Audit Found

An independent architectural review identified the following (ordered by severity):

### High — Operational immaturity
The runtime is a thin scaffold. Network behavior, adversarial I/O, persistence,
crash recovery, and synchronization have not been exercised. The protocol design
is rigorous; the operational system is pre-production. This is expected at this
stage and explicitly documented, but must be resolved before any deployment.

**Target milestone:** PAL Host implementation with real network, storage, and
recovery; end-to-end integration test suite.

### Potentially High — Halt-trigger fragility
The absorbing-halt semantic is philosophically correct and formally proved (TH-4
through TH-8). However, systems with deterministic halt-on-divergence can be
vulnerable to: malformed edge-case inputs triggering halt, validator liveness
suppression, and economic griefing through intentional divergence injection.
The proofs cover correctness of the halt mechanism, not resistance to adversarial
activation of it.

**Target milestone:** Adversarial simulation suite targeting halt-trigger paths;
formal analysis of minimum input required to trigger halt; grief-cost analysis.

### Medium — Proof claims exceed visible audit trail
The Coq evidence trail is now partially automated: the CI `proofs` job installs
Coq, records the Coq version, rejects active `Admitted`/`admit` markers, checks
new axiom declarations against `proofs/COVERAGE.md`, compiles the active Coq
proof set with `coqc`, records `.vo` SHA-256 hashes, and uploads the version and
hash manifests as artifacts. The remaining visible-audit gaps are proof-to-code
refinement, extraction equivalence documentation, and a reproducible build
environment that lets independent auditors recreate the same proof objects.

**Target milestone:** proof coverage matrix linked to specific Coq theorems with
commit-pinned CI artifact hashes; extraction pipeline documentation; formal
refinement proof between the Coq model and Rust runtime.

### Medium — Fixed-size structures create scaling and pressure concerns
`MAX_VALIDATORS = 1024` with large fixed arrays, full-state copies per epoch,
and stack-heavy structures may create stack exhaustion risks, cache inefficiency,
serialization amplification, and replay cost spikes — even if all values are
formally deterministic.

**Target milestone:** Benchmark suite measuring worst-case epoch transition
memory footprint, serialization throughput, stack growth under nested transitions,
and replay latency. Results archived under `artifacts/benchmarks/`.

### Medium — Large trusted computing base without attestation
Correctness reduces to ISA correctness, Rust compiler correctness, and AX-3
(cryptographic assumption). That is mathematically honest but operationally the
compiler/toolchain trust boundary is large. There is no reproducible build
framework, binary transparency mechanism, or multi-compiler validation.

**Target milestone:** Reproducible build environment (Nix flake or pinned Docker);
byte-identical release attestations; dual-compiler differential testing (at minimum
two different LLVM backend configurations).

---

## Strategic Next Steps (Prioritized)

### Phase 1 — Pre-genesis lock (current)

These must be completed before `GENESIS_CONSTANTS.toml` is locked. Everything
here is Domain A correctness work.

1. **Discharge open proof obligations** (`proofs/COVERAGE.md`):
   - TH-10: Cascade collision resistance (`cascade/cascade_collision_resistance.v`)
     — post-genesis migration item; v1.0 Domain A state roots remain `H_domain` / SHA3-256. Full activation requires formalising hash function; consider EasyCrypt or CryptHOL
   - Blinding PRF: `H_cascade_keyed` is a PRF — formal proof in CryptHOL/SSProve
   - IT-MAC: GF(2¹²⁸) forgery bound — mechanise in Coq via GHASH polynomial MAC reduction

2. **Proof CI pipeline**:
   - Keep the existing `.github/workflows/ci.yml` `proofs` job green: Coq install,
     version capture, admitted-marker rejection, axiom coverage checking, active
     Coq compilation, proof-object hashing, and artifact upload.
   - Extend the artifact trail with commit-pinned retention/indexing so auditors
     can reproduce and compare proof-object hashes outside GitHub Actions.
   - Decide whether `_wip/` drafts and future proof trees need separate non-gating
     CI coverage.

3. **Deep audit of core Domain A modules** (target: external cryptographer review):
   - `fixed_point.rs` — overflow handling, saturation behavior, rounding invariants
   - `encoding.rs` — injectivity edge cases, malformed input handling, canonicalization
   - `lyapunov.rs` — monotonicity assumptions, adversarial convergence behavior
   - `hash.rs` / `cascade.rs` — constant-time concerns, domain separation correctness
   - `transaction.rs` — nonce handling, validator slot invariants, reordering resistance

4. **Fuzz coverage expansion** ✓ COMPLETE:
   - `fixed_point_fuzz` added: exercises overflow/saturation boundaries
   - `encoding_fuzz` added: validates decode robustness + encode/decode roundtrip invariants
   - `lyapunov_fuzz` added: stresses monotonicity-related invariants under arbitrary inputs
   - `transition_fuzz` retained in CI smoke suite for halt-trigger-path pressure testing

### Phase 2 — Operational hardening

These are prerequisites for any deployment, testnet or otherwise.

5. **PAL Host implementation**:
   - Real network transport (TCP/UDP; P2P gossip)
   - Persistent state storage (crash-safe WAL or equivalent)
   - Crash recovery with replay-from-genesis verification
   - Integration test suite exercising Domain B → Domain A boundary

6. **Performance characterization**:
   - Benchmark worst-case epoch transition (1024 validators, max divergence)
   - Measure serialization throughput and stack depth
   - Profile replay latency against spec requirements (450 ms control-loop budget)
   - Archive results under `artifacts/benchmarks/`

7. **Adversarial simulation**:
   - Halt-trigger test suite: minimum input cost to trigger absorbing halt
   - Liveness suppression simulation: can a minority validator coalition freeze progress?
   - Economic grief analysis: cost/benefit of intentional divergence injection
   - Reorg/replay attack surface review (even without PoW/PoS, replay vectors exist)

### Phase 3 — Assurance hardening

These are required for independent auditability and long-term trust.

8. **Reproducible builds** ✓ COMPLETE:
   - `docker/Dockerfile.build` pins Rust 1.95.0 + Coq on `debian:bookworm-20250407-slim`
   - `release-attestation.yml` CI job: two-stage byte-identical build verification, SHA-256 manifest recorded under `artifacts/attestations/` and uploaded as 365-day CI artifact
   - `scripts/attest_release.sh` for local/Docker reproduction
   - `rust-toolchain.toml` pins Rust 1.95.0; `scripts/verify_rust_toolchain.sh` verifies in every CI job

9. **Proof-to-code refinement** ✓ COMPLETE:
   - `proofs/model/RefinementStatement.v`: RT-1 … RT-4 formally proved; AX2_rust_refinement axiom with documented justification; `rust_RT1` … `rust_RT4` corollaries
   - `proofs/model/Extract.v`: Coq extraction pipeline to OCaml (manual; not CI-compiled)
   - `docs/refinement.md`: three-layer correspondence chain, Coq-to-Rust definition mapping, extraction usage, axiom stack, and strengthening roadmap
   - Coverage: 18 PROVED, 4 CI-VERIFIED, 3 AXIOM, 2 PLACEHOLDER (see `proofs/COVERAGE.md`)

10. **Multi-compiler differential testing** ✓ COMPLETE:
    - `.github/workflows/multi-compiler-diff.yml`: scheduled weekly CI job with two gates:
      (a) REQUIRED — `opt-level=0` vs `opt-level=3` on stable rustc 1.95.0, state roots must be identical;
      (b) ADVISORY — cranelift nightly backend vs LLVM baseline (`continue-on-error: true`)
    - `scripts/run_differential_corpus.sh`: local differential check script
    - Divergence between opt levels = UB in Domain A code — investigation required before any release

---

## What Is Not Changing

These are fixed constraints that no future work will alter:

- `GENESIS_CONSTANTS.toml` is append-only until genesis lock; after lock it is immutable.
  Any change defines a new network.
- Domain A (`crates/consensus`) forbids: `unsafe`, `f32`/`f64`, `usize`/`isize` in
  state fields, `HashMap`, wall clock, OS entropy, unchecked arithmetic.
- All arithmetic overflow in Domain A triggers absorbing halt — not panic, not saturation.
- Every new transaction type requires a filed proof obligation on its effect on
  `δ_window` before any implementation is merged.
- Cross-ISA replay invariance (TH-7) is a non-negotiable CI gate.

---

## Proof Coverage Summary (current)

| Status | Count | Meaning |
|--------|-------|----------|
| PROVED | 18 | Coq theorem, compiles, zero `Admitted` |
| CI-VERIFIED | 4 | Verified by cross-ISA CI or KAT vectors |
| AXIOM | 3 | Assumed property rows with documented justification; not provable from first principles |
| PLACEHOLDER | 2 | Coq file exists, body axiomatised or reduction target deferred; full proof deferred |
| MISSING | 0 | |

Full matrix: `proofs/COVERAGE.md`

---

## Key Files for Reviewers

| File | Purpose |
|------|----------|
| `README.md` | Project identity, theorem table, contributor rules |
| `design_decisions.md` | Architectural decisions and rationale |
| `GENESIS_CONSTANTS.toml` | All protocol parameters (immutable after lock) |
| `docs/spec/00_execution_model.md` | Domain A/B partition, execution constraints |
| `docs/spec/01_consensus.md` | State space, encoding, transition function |
| `docs/spec/07_hash_cascade.md` | 8-family cascade spec (post-genesis/v1.1); not the v1.0 state-root commitment |
| `docs/traceability.md` | PDF → code → test → proof audit contract |
| `proofs/COVERAGE.md` | Full proof obligation matrix |
| `proofs/STATUS.md` | Per-file Coq compilation status |
| `crates/consensus/src/transition.rs` | Core state transition function |
| `crates/consensus/src/encoding.rs` | Canonical serialization |
| `crates/consensus/src/fixed_point.rs` | Fixed-point arithmetic (audit target) |
| `crates/consensus/src/lyapunov.rs` | Lyapunov stability evaluation |
| `fuzz/fuzz_targets/` | Fuzz harnesses for Domain A functions |
| `.github/workflows/` | CI pipeline definitions |


## Compliance readiness scorecard

This scorecard is populated from CI-generated compliance artifacts (see `.github/workflows/compliance-evidence.yml`).

| Evidence class | CI artifact source | Current status |
|---|---|---|
| Cryptographic KATs | KAT test artifacts from CI test jobs | Pending integration |
| DRBG health/self-tests | Self-test logs from CI runtime checks | Pending integration |
| Reproducible build attestations | Provenance/reproducibility artifacts under `artifacts/compliance/provenance/` | Skeleton workflow added |
| SBOM + vuln scan | `artifacts/compliance/sbom/`, `artifacts/compliance/vuln/` | Skeleton workflow added |
| Proof coverage report | `proofs/COVERAGE.md` + CI-uploaded proof hashes/manifests | Available; wiring to scorecard pending |

Scoring policy:
- Green: evidence present, policy-compliant, and indexed.
- Yellow: evidence generated but policy disposition pending.
- Red: required evidence missing or stale.

Release policy reminder: no release tag is permitted without a complete `artifacts/compliance/<tag>/index.json` evidence index.
