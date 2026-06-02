# QASH — Project Status and Strategic Roadmap

> **Audience:** External auditors, potential contributors, and technical reviewers.
> This document gives an honest account of what has been built, what the open gaps
> are, and what the priority order for closing them is. It is updated as milestones
> are reached.
>
> Last updated: 2026-05-29. Based on internal review, the current CI workflow,
> and an independent external audit of the architecture, workspace, and consensus
> implementation.
>
> **Genesis posture:** pre-genesis integration RC. `GENESIS_CONSTANTS.toml`
> remains provisional and non-authoritative; this status update is not a genesis
> lock recommendation.

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
| Formal proof coverage | **Strong, pre-genesis** | TH-3 local arithmetic plus executable-step closure, TX-0/TX-1 perturbation proofs, EFB determinism, refinement statement, and extraction surface are checked by `make -C proofs all`; see `proofs/STATUS.md` and `proofs/COVERAGE.md` |
| Proof CI pipeline | **Automated for active Coq proofs** | `.github/workflows/ci.yml` installs Coq, rejects active `Admitted`/`admit` markers, checks new axioms against `proofs/COVERAGE.md`, compiles active `.v` files including `model/Extract.v`, records `.vo` SHA-256 hashes, and uploads version/hash artifacts |
| Runtime / PAL implementation | **Integration complete (v1.0 scope)** | Receipt encryption is ChaCha20-Poly1305 AEAD; XOR stub deleted. WAL crash-recovery robustness hardened with fuzz target + 5 integration tests. Domain B backend scope classified (v1_domain_b_backend_boundary.md, ADR-013). Production networking, hardware attestation (TPM/TDX/CCA/SEV-SNP), threshold signing, and Plonky3 are post-v1 and feature-gated. |
| Fuzzing infrastructure | **Expanded (Wave 4 PR #233)** | honggfuzz harness covers encoding, decode, transition, fixed-point, lyapunov, cascade, tx, and WAL-decode targets; 5 replay-robustness integration tests; fuzz-smoke CI gate runs all targets |
| Performance benchmarks | **Implemented (Wave 5 PR #235)** | Criterion suites in `crates/consensus/benches/epoch_transition.rs` and `crates/pal/benches/dual_hash.rs` cover: worst-case epoch transition (1024v, max divergence), tx-heavy advance (all-1024v TX batch forward+reverse), full-state encode/decode, N-epoch replay, hash cascade, tx admission latency, validator lookup, state-root commitment, and all-of manifest overhead. `max_validators_state_copy` benchmark added in PR #235. Archive results with `cargo bench -- --output-format bencher 2>&1 \| tee artifacts/benchmarks/$(date -u +%Y%m%dT%H%M%SZ).txt`. Phase 2-R optimisation is reserved (PR #236, conditional on bottleneck evidence). |
| Reproducible builds | **Done** | `rust-toolchain.toml` pins 1.95.0; `docker/Dockerfile.build` pins full build+proof environment; `release-attestation.yml` CI job verifies byte-identical two-stage builds and records SHA-256 manifests under `artifacts/attestations/` with 365-day retention |
| Adversarial simulation | **Done** | 23-test suite across 10 scenarios: halt-trigger boundary, liveness suppression, coordinated spike, nonce replay, max-field saturation, slash monotonicity, halt irreversibility, grace period |
| Deep module audits | **Done** | `fixed_point.rs`, `encoding.rs`, `lyapunov.rs`, `hash.rs`, `transaction.rs` audited and hardened with boundary/adversarial tests (PRs #69, #71) |
| Multi-compiler differential | **Done** | opt-level=0 vs opt-level=3 required gate + cranelift advisory; weekly scheduled CI (PR #65) |
| Genesis / production readiness | **Pre-genesis RC, not locked** | Protocol evidence is converging, but genesis lock is blocked on the missing normative PDF tracked in issue #209. Traceability artifact reconciliation, final genesis hash lock, release sign-off, and production PAL decisions remain gated on that artifact. |

## Current Post-Merge State

As of 2026-06-01 (genesis-candidate evidence waves):

- `main` includes all-of cleanup from PR #225 (post-allof baseline).
- Genesis-candidate evidence is being assembled in waves on PR #226:
  - Wave 0: post_allof_baseline.md; PR #217 deferred
  - Wave 1: PDF traceability verified (Phase 1-D complete)
  - Wave 2: Axiom classification (v1_axiom_boundary.md); Coq↔Rust parity extended to 12 vectors (TV-0..TV-11); TLA+ advisory errata
  - Wave 3: Receipt encryption upgraded to ChaCha20-Poly1305 AEAD (XOR removed); Domain B stub register and backend boundary classified
  - Wave 4: WAL fuzz target (wal_decode.rs); replay robustness integration tests (5 tests); ADR-013 backend boundary; cross-ISA WAL CI steps
  - Wave 5: Benchmark evidence suite complete; max_validators_state_copy bench added
  - Wave 6 (in progress): Stale docs reconciliation, compliance pass
- `cargo test --workspace --no-default-features` passes.
- `cargo test --workspace --features std` passes.

---

## Strategic Execution Order

1. Land the current integration work in reviewable slices: sharding/EFB scaffold,
   PAL whole-protocol scaffold, proof/refinement closure, and PR #93 hygiene.
   The current slice map is `docs/release/current_integration_review_slices.md`.
2. Use `docs/release/pre_genesis_evidence_snapshot.md` as the audit handoff for
   local commands, allowed claims, blocked claims, and evidence capture via
   `scripts/capture_pre_genesis_evidence.sh`.
3. Start Phase 2-R runtime optimization only after tx-heavy benchmarks and
   parity tests exist.
4. Keep production PAL/ZK backend work separate from Phase 2-R.
5. Delay any genesis-lock/reference-tag decision until the normative PDF,
   traceability, genesis hash, release evidence, and owner sign-off are
   reconciled.

---

## PR #93 Review Incorporation

The PR #93 raw transcript branch adds a root file named `21`. That transcript is
not a repository artifact for this branch. Review findings are instead extracted
into canonical docs, tests, proofs, and implementation files.

Incorporated now:
- Sharding is documented and scaffolded as protocol structure, not merely a
  module.
- The PR template and CI document-hygiene job reject raw transcript dumps and
  ad hoc root-level spec files.
- The ZK profile is fixed as Plonky3 FRI-STARK with Poseidon circuit hashing,
  QASH-native public commitments, and a two-layer recursion profile.
- EFB roots, aggregate receipt roots, whole-protocol PAL replay, and sharded
  replay vectors are represented in the current integration scaffold.

Scheduled, not implemented in this detour:
- `Phase 2-R: Core Runtime Optimization`, covering single-pass transaction
  admission, deterministic total-order sorting, streaming state-root hashing,
  `ProjectedView`, and optional validator-directory work. These changes must
  preserve consensus bytes exactly and pass vector, cross-ISA, and benchmark
  gates before they support performance claims.

---


## PR #93 Gap Closure Plan (Delta From Draft Comments)

The PR #93 draft feedback is partly implemented and partly scheduled. The
remaining delta, compared against canonical repository artifacts, is:

- **Implemented now:** sharding protocol scaffold, fixed provisional ZK profile,
  transcript/document hygiene, and explicit Phase 2-R scheduling.
- **Still open:** Phase 2-R code-path execution, production Plonky3 verifier
  backend in PAL, cross-ISA hosted replay expansion for sharded traces, and
  benchmark-archived evidence for performance claims.

This delta is intentional: no runtime optimization or production-verifier claim
is considered complete until parity/benchmark gates are captured as artifacts
for the exact reviewed commit.

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

### Phase 1 — Pre-genesis integration RC (current)

These items convert the implemented components into an audit-ready pre-genesis
snapshot without locking `GENESIS_CONSTANTS.toml`.

1. **Stabilize proof evidence**:
   - Keep TH-3 composition closure checked in `proofs/composition/th3_system_closure.v`.
   - Keep TX-0/TX-1 perturbation obligations checked before adding TX-2.
   - Keep EFB determinism and model extraction in the `make -C proofs all` gate.

2. **Proof CI pipeline**:
   - Keep the existing `.github/workflows/ci.yml` `proofs` job green: Coq install,
     version capture, admitted-marker rejection, axiom coverage checking, active
     Coq compilation, proof-object hashing, and artifact upload.
   - Extend the artifact trail with commit-pinned retention/indexing so auditors
     can reproduce and compare proof-object hashes outside GitHub Actions.
   - Treat Kani as advisory until CI install/runtime behavior is repeatable, then
     promote selected harnesses to a required gate.

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
   - Keep the current hosted replay and whole-protocol harnesses green.
   - Add real network transport and hardware-backed attestation behind Domain B
     interfaces without feeding nondeterminism into Domain A.
   - Replace the static ZK proof-bundle verifier with a Plonky3 FRI-STARK
     backend that preserves the fixed two-layer recursion profile.
   - Harden crash recovery with replay-from-genesis verification before any
     deployment claim.

6. **Performance characterization**:
   - Benchmark worst-case epoch transition (1024 validators, max divergence)
   - Measure serialization throughput and stack depth
   - Profile replay latency against spec requirements (450 ms control-loop budget)
   - Archive results under `artifacts/benchmarks/`

6a. **Core runtime optimization (Phase 2-R, scheduled)**:
   - Single-pass transaction admission with deterministic `Candidate` records.
   - Deterministic total-order sorting by `(sort_key, tx_id)`.
   - Streaming state-root commitment with exact preimage parity against the
     current buffered encoder.
   - Runtime-only `ProjectedView` to reduce full-state copies without changing
     the Coq logical transition model.
   - Optional validator directory only if tx-heavy profiling shows lookup cost
     dominates and the sidecar is rebuilt deterministically per epoch.

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

9. **Proof-to-code refinement** ✓ PRE-GENESIS EVIDENCE READY:
   - `proofs/model/RefinementStatement.v`: RT-1 … RT-4 formally proved; AX2_rust_refinement axiom with documented justification; `rust_RT1` … `rust_RT4` corollaries
   - `proofs/model/Extract.v`: Coq extraction pipeline to OCaml, checked by `make -C proofs all`
   - `docs/refinement.md`: three-layer correspondence chain, Coq-to-Rust definition mapping, extraction usage, axiom stack, and strengthening roadmap
   - Coverage: 42 PROVED, 4 CI-VERIFIED, 3 AXIOM, 6 PLACEHOLDER (see `proofs/COVERAGE.md`)

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
- Genesis lock is not implied by proof readiness; authoritative deployment requires
  a separate lock decision and guarded artifact update.

---

## Proof Coverage Summary (current)

| Status | Count | Meaning |
|--------|-------|----------|
| PROVED | 42 | Coq theorem, compiles, zero `Admitted` |
| CI-VERIFIED | 4 | Verified by cross-ISA CI or KAT vectors |
| AXIOM | 3 | Assumed property rows with documented justification; not provable from first principles |
| PLACEHOLDER | 6 | Coq file exists, body axiomatised or reduction target deferred; full proof deferred |
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
