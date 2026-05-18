# QASH — Project Status and Strategic Roadmap

> **Audience:** External auditors, potential contributors, and technical reviewers.
> This document gives an honest account of what has been built, what the open gaps
> are, and what the priority order for closing them is. It is updated as milestones
> are reached.
>
> Last updated: 2026-05-18. Based on internal review, the current CI workflow,
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
| Formal proof coverage | **Strong conceptually** | 14 PROVED, 4 CI-VERIFIED, 2 AXIOM, 2 PLACEHOLDER — see `proofs/COVERAGE.md` |
| Proof CI pipeline | **Automated for active Coq proofs** | `.github/workflows/ci.yml` installs Coq, rejects active `Admitted`/`admit` markers, checks new axioms against `proofs/COVERAGE.md`, compiles active `.v` files, records `.vo` SHA-256 hashes, and uploads version/hash artifacts |
| Runtime / PAL implementation | **Scaffold only** | PAL Host returns zeroes/no-ops; no network, no persistence, no crash recovery |
| Fuzzing infrastructure | **Basic** | honggfuzz harness for 3 Domain A targets; fuzz-smoke CI gate passes |
| Performance benchmarks | **None** | Worst-case epoch transition cost, serialization throughput, stack depth: unmeasured |
| Reproducible builds | **Partial** | Rust is pinned to 1.95.0 with CI version verification and local locked/offline build verification; no Nix/Docker environment or byte-identical release attestation yet |
| Adversarial simulation | **None** | Halt-trigger griefing, liveness suppression, economic griefing: untested |
| Deep module audits | **Pending** | `fixed_point.rs`, `encoding.rs`, `lyapunov.rs`, `hash.rs` not yet independently audited |
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

4. **Fuzz coverage expansion**:
   - Add `fixed_point_fuzz` target: exercise overflow/saturation boundaries
   - Add `encoding_fuzz` target: verify decode never panics + encode/decode roundtrip
   - Add `lyapunov_fuzz` target: verify monotonicity invariant under arbitrary inputs
   - Extend `transition_fuzz` to cover halt-trigger edge cases explicitly

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

8. **Reproducible builds**:
   - Nix flake or pinned Docker image for all build/proof tooling
   - Byte-identical release attestation CI job
   - `rust-toolchain.toml` pins Rust 1.95.0 for consensus, fuzz smoke, CI build/test/lint jobs, with `rustc --version --verbose` verification

9. **Proof-to-code refinement**:
   - Formal refinement proof between `proofs/model/Model.v` and `crates/consensus/`
   - Coq extraction pipeline producing executable Rust skeleton verified against `crates/`
   - Document trusted axiom minimization (reduce dependence on AX-2)

10. **Multi-compiler differential testing**:
    - Build consensus crate with two different LLVM configurations
    - Build with `cranelift` backend as differential oracle
    - Cross-check state roots from each build on identical input corpus

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
| PROVED | 14 | Coq theorem, compiles, zero `Admitted` |
| CI-VERIFIED | 4 | Verified by cross-ISA CI or KAT vectors |
| AXIOM | 2 | Assumed property rows with documented justification; not provable from first principles |
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
