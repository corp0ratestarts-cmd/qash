# QASH Protocol

![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)
![Cross-Platform Determinism](https://github.com/<OWNER>/qash/actions/workflows/platform-determinism.yml/badge.svg)
![QASH CI](https://github.com/<OWNER>/qash/actions/workflows/ci.yml/badge.svg)

---

## What QASH Is

QASH is a **deterministic replicated transition calculus** with post-quantum
cryptographic anchoring and formally machine-checkable safety properties.

The central architectural invariant is:

> **Identical replay produces identical state.**
>
> All subsystems are subordinate to this guarantee.

QASH is **not**:
- A probabilistic consensus chain
- A Nakamoto-style longest-chain system
- A validator lottery or leader election system
- A speculative execution VM
- A conventional blockchain node

QASH **is**:
- A deterministic replicated state machine
- A replay-verifiable execution environment
- A formally constrained transition system whose safety properties
  are intended to be machine-provable from explicit axioms
- A protocol where serialization is ontology: two states that encode
  identically are the same state

---

## Genesis Lock Status

> **Pre-lock:** `QASH_Spec_v1.0.pdf` has not yet been committed to `spec/pdf/`.
> All quotes, page references, and requirement traces in `docs/traceability.md`,
> `docs/errata/`, and `docs/adr/` are **provisional** until the PDF is committed
> and the genesis hash is recomputed. See `spec/pdf/README.md` for the full
> authority rule and lock procedure.

---

## Repository Structure

```
spec/pdf/           ← Normative PDF source of truth (v1.0, checked in before genesis lock)
docs/traceability.md ← PDF requirement → code → test → proof contract
docs/errata/         ← Normative corrections/clarifications to the PDF
docs/adr/            ← Engineering decisions and PDF-silent gap definitions
docs/spec/           ← Pre-existing derived engineering specs pending mirror migration
  00_execution_model.md   Deterministic execution substrate
  01_consensus.md         State space, encoding, transition function, stability
  07_hash_cascade.md      Astronomical depth-7 cascade spec (v1.1)
  09_migration_v1.0_to_v1.1.md  Migration guide and compatibility window
  12_sharded_protocol.md  Sharding, EFB, cross-shard receipts, ZK profile

proofs/             ← Formal theorems (Coq)
  contractivity/
    lyapunov_stability.v  TH-3a/TH-3b/TH-3c foundation proofs
    tx_perturbation_0.v   TX-0 §A8 Form A proof obligation
  util/
    list_inj.v            fixed-width list/encoding support lemmas
  _wip/
    encode_injectivity.v.draft  TH-1/TH-2 draft, not genesis-lock evidence
    absorbing_halt.v.draft      TH-4/TH-5/TH-6/TH-8 draft, not CI-gated

model/              ← Canonical executable semantics (extracted from proofs)
  README.md               Model contract and extraction notes

crates/
  consensus/        ← no_std consensus core (Domain A)
  pal/              ← Platform Abstraction Layer (Domain B)

src/                ← Hosted binary entrypoint
GENESIS_CONSTANTS.toml   Immutable genesis parameters (not yet locked)
```

> **Runtime status: integration scaffold — hosted PAL replay, commitment
> transport, attestation verifier interfaces, whole-protocol sharded replay, and
> a ZK proof-bundle boundary exist. Production networking, hardware attestation,
> Plonky3 proof verification, and certification evidence are not deployable yet.**
>
> **Evidence handoff:** current pre-genesis claims, blocked claims, and local
> verification commands are tracked in
> `docs/release/pre_genesis_evidence_snapshot.md`.

The relationship between layers:

```
spec/pdf/     = normative source of truth      (what the protocol INTENDS)
docs/traceability.md = audit contract          (what is mapped to code/tests/proofs)
docs/errata/  = explicit PDF corrections       (what changes/clarifies the PDF)
docs/adr/     = engineering decisions          (how PDF gaps are filled)
proofs/       = formal guarantees             (what is PROVED about it)
model/        = canonical executable model    (what it COMPUTES, extracted from proofs)
crates/       = optimized implementation      (what is DEPLOYED)
```

The runtime (`crates/`) must be observationally equivalent to the model
for all admissible inputs. This equivalence is a future formal proof target.

The prior `docs/spec/` documents remain useful engineering specs, but the
repository now resolves authority through the PDF-first governance model in
`docs/traceability.md`: PDF quote → erratum/ADR if needed → code → test/vector
→ proof.

## Local Verification Setup

Install the local tools used by the Rust, Coq, Kani, fuzz-smoke, and
cross-ISA replay lanes:

```
scripts/install_test_dependencies.sh
```

The installer provisions the apt packages, pinned Rust toolchain targets, and
Cargo tools mirrored from CI (`cargo-deny`, `honggfuzz`, and Kani 0.67.0). It is
intended for Debian/Ubuntu-style hosts and uses `sudo apt-get` when it is not
run as root. Set `SKIP_APT=1` when system packages are already present or must
be installed by a separate privileged session.

---

## Theorem Status

| ID | Name | Class | Status |
|----|------|-------|--------|
| TH-1 | Encoding injectivity | Formal theorem | ✅ FORMAL — `proofs/contractivity/encode_injectivity.v` |
| TH-2 | Encoding totality | Formal theorem | ✅ FORMAL — `proofs/contractivity/encode_injectivity.v` |
| TH-3 | Convergence decrease / halt gate | Formal theorem | ✅ FORMAL — `proofs/contractivity/lyapunov_stability.v` + `proofs/composition/th3_system_closure.v` |
| TX-0 §A8 | No-op perturbation bound | Formal theorem | ✅ FORMAL — `proofs/contractivity/tx_perturbation_0.v` |
| TX-1 §A8 | Score-decrement perturbation bound | Formal theorem | ✅ FORMAL — `proofs/contractivity/tx1_score_decrement.v` |
| TH-4 | Φ_safety monotonicity | Formal theorem | ✅ FORMAL — `proofs/safety/absorbing_halt.v` |
| TH-5 | Φ_safety boundedness | Formal theorem | ✅ FORMAL — `proofs/safety/absorbing_halt.v` |
| TH-6 | Halt correctness | Formal theorem | ✅ FORMAL — `proofs/safety/absorbing_halt.v` |
| TH-7 | Replay invariance RT-1 | Verification claim | ✅ CI-VERIFIED — identical state roots on x86_64, aarch64, riscv64gc (QEMU user-static) |
| TH-8 | Succession soundness | Formal theorem | ✅ FORMAL — `proofs/safety/absorbing_halt.v` + `proofs/integration/th8_composition.v` |
| Sharding/EFB | EFB determinism and receipt anchoring | Mixed | ✅ SCAFFOLDED — `docs/spec/12_sharded_protocol.md`, `crates/consensus/src/sharding.rs`, `proofs/sharding/efb_determinism.v` |

Genesis lock gate:
- TH-1 through TH-6, TH-8: **FORMAL** (Coq compiles; no `Admitted` beyond AX-1/AX-2/AX-3)
- TH-7: CI-verified on x86_64, aarch64, and riscv64gc (QEMU user-static; identical state roots)
- Archived drafts in `proofs/_wip/` are superseded — not lock evidence
- Genesis remains unlocked until traceability, normative PDF, production PAL,
  and release evidence are reconciled.

---

## Sharding and ZK Profile Status

Sharding is protocol structure, not an optional implementation module. The
current v1.2 scaffold includes deterministic shard assignment, cross-shard
receipt IDs, EFB aggregation, EFB roots in `PublicTranscript`, and replay
vectors. The provisional ZK profile is fixed as Plonky3 FRI-STARK with Poseidon
inside the circuit and QASH-native outer commitments, using a two-layer
recursion tree: Layer 0 shard validity, Layer 1 16:1 aggregation, Layer 2 EFB
verification.

This is not a production ZK verifier. Domain A validates the public profile
shape and commits to `zk_batch_root`; Domain B owns proof generation,
proof-byte transport, and the future Plonky3 verifier backend.

## PR #93 Review Status

PR #93 review feedback is incorporated through curated repository artifacts, not
by tracking the raw conversational transcript file (`21`) from that PR branch.
Protocol material extracted from the review belongs in `docs/spec/`,
`docs/adr/`, `ROADMAP.md`, `PROJECT_STATUS.md`, tests, and proofs.
The CI `document-hygiene` job rejects obvious raw transcript dumps and ad hoc
root-level spec files so this remains enforceable after this branch.

The sharding/ZK comments are reflected in the v1.2 scaffold: sharding is part
of protocol structure, the provisional proof profile is Plonky3 FRI-STARK with
Poseidon inside the circuit and QASH-native public commitments outside it, and
the intended recursion profile is Layer 0 shard validity, Layer 1 16:1
aggregation, and Layer 2 EFB verification.

The latest runtime-performance review is scheduled as `Phase 2-R: Core Runtime
Optimization` in `ROADMAP.md` and `docs/adr/ADR-006-runtime-optimization-track.md`.
It is not implemented in this detour; it is constrained to consensus-byte-
preserving refactors with parity and benchmark gates.

---


## PR #93 Gap Matrix (Draft Review vs Current Repo)

The PR #93 draft comments are treated as requirements only when they can be
mapped to canonical repo artifacts. The table below tracks that mapping and
highlights the remaining gaps.

| PR #93 comment theme | Already incorporated | Gap / required follow-up |
|---|---|---|
| Sharding must be protocol structure, not a loose module | `docs/spec/12_sharded_protocol.md`, `crates/consensus/src/sharding.rs`, v1.2 vectors and replay tests | Expand cross-ISA hosted replay evidence for sharded whole-protocol traces before production claims |
| ZK profile must be fixed and auditable | Provisional profile fixed as Plonky3 FRI-STARK + Poseidon-in-circuit + QASH commitments, documented in spec/roadmap | Implement production Plonky3 verifier backend in Domain B with profile-lock tests and artifacted performance evidence |
| Runtime hot-path inefficiencies should be addressed without semantic drift | ADR-006 and Phase 2-R scheduling with parity preconditions exist | Execute Phase 2-R implementation behind strict consensus-byte parity, cross-ISA parity, and benchmark archive gates |
| Raw conversational transcript material should not become canonical spec text | CI/document hygiene checks and PR template guards exist | Keep enforcing canonical placement (`docs/spec`, `docs/adr`, proofs, tests) and reject ad hoc root artifacts |
| Performance claims must be evidence-backed | Precondition tests + bench compilation gates exist | Publish tx-heavy and commit-path benchmark artifacts before any latency/finality claims |

## Planned Future Work (PR #93 Follow-Through)

The next track is explicitly limited to closing the remaining PR #93 gaps:

1. **Phase 2-R implementation** (single-pass admission, deterministic total-order sorting, streaming root hashing, runtime `ProjectedView`).
2. **Profile-locked Plonky3 verifier backend in PAL** (Domain B only, no Domain A semantic influence).
3. **Cross-ISA hosted replay evidence expansion** for sharded whole-protocol traces.
4. **Benchmark publication discipline** under `artifacts/benchmarks/` for all performance-facing claims.
5. **Documentation hygiene continuity** so review conclusions remain encoded as auditable repo artifacts.

## Foundational Axioms

All guarantees reduce to three axioms. Everything above them is deductively certain.

```
AX-1  Authorized ISAs implement two's complement arithmetic correctly
AX-2  Pinned Rust toolchain produces correct code for authorized ISAs
AX-3  Active consensus hash suite is collision-resistant  (cryptographic assumption, not theorem)
```

---

## `GENESIS_CONSTANTS.toml` Is Immutable

Once locked, `GENESIS_CONSTANTS.toml` cannot be modified.
Any change requires a new network. There are no protocol upgrades, governance
votes, or emergency patches. This is a design property, not a limitation.

---

## Contributing

QASH implementation discipline is closer to **kernel development** or
**avionics software** than conventional blockchain development.

### The non-negotiable rules for Domain A (consensus core)

```
FORBIDDEN in crates/consensus/ and anything it calls:
  - f32, f64, or any floating-point type
  - HashMap or HashSet (use BTreeMap, BTreeSet)
  - panic!(), unwrap(), expect()  — use explicit match + absorbing_reset()
  - unsafe blocks
  - std::time (wall clock)
  - OS randomness
  - nondeterministic iteration order
  - usize or isize  (use explicit u32/u64/i64/i128)
  - Rust default / on signed integers for protocol arithmetic  (use div_euclid)
  - Any heap allocation without statically-bounded size
```

### Domain B (PAL) rules

```
PERMITTED in crates/pal/:
  - unsafe under formal audit and review
  - SIMD and hardware acceleration
  - OS networking, clocks, entropy
  - Dynamic allocation

FORBIDDEN in crates/pal/:
  - Any Domain B value influencing Domain A state transitions
  - Clock or entropy inputs to the consensus execution path
```

### Every PR must

- Pass `cargo test --no-default-features` (consensus core)
- Pass `cargo build -p qash-pal --features std`
- Pass cross-ISA determinism check (automated in CI)
- Not introduce `Admitted` to any proof file without an explicit
  tracking issue and mathematical justification

### Contribution philosophy

> Nothing gets coded until it has a corresponding definition in the spec.
> No transaction type enters the protocol unless its effect on δ_window
> has a formal proof obligation filed.

New features that cannot be proved to preserve the convergence invariant
will not be merged, regardless of utility. This is the cost of formal
replay guarantees. It is also the source of QASH's long-term value.

---

## Spec Version Binding

The spec version is content-addressed:

```
spec_hash = SHA3-256(00_execution_model.md ∥ 01_consensus.md)
```

This hash is recorded in `GENESIS_CONSTANTS.toml` at genesis lock time.
Any runtime must declare the spec hash it implements. Mismatches are
flagged by CI as non-conforming, regardless of test passage.

---

## Patent Evidence Pack

The repository now includes a patent-support evidence structure for technical
review with qualified counsel. These materials are not legal advice; they
organize implementation-specific evidence around candidate invention families:

- deterministic replay isolation architecture,
- Lyapunov-based validator stability evaluation,
- cross-ISA deterministic reproducibility enforcement,
- prior-art differentiation working notes,
- claim-support traceability,
- replay and benchmark artifact templates,
- nondeterminism threat modeling, and
- architecture decision records.

Start at `patents/README.md`. Replay evidence should be archived under
`artifacts/replay_equivalence/`, and technical-effect measurements should be
archived under `artifacts/benchmarks/`.

---

*QASH is licensed GPL-3.0-or-later.*
*`GENESIS_CONSTANTS.toml` will be immutable after genesis lock.*
*Modifying it requires a new network.*

---

## Certification-Grade Delivery Path (Phase-by-Phase)

This section defines a certification-oriented execution path intended for high-assurance review contexts (formal methods, safety/security evaluation, and independent reproducibility audits).

### Assurance Objectives

QASH should be advanced under explicit assurance goals:

1. **Deterministic Safety Objective**: identical admissible replay yields identical state roots across authorized ISAs.
2. **Semantic Correctness Objective**: Rust consensus behavior is observationally equivalent to canonical model semantics for the declared refinement surface.
3. **Cryptographic Boundary Objective**: every cryptographic claim is either formally proved, reduced to an explicit axiom, or marked as deferred with bounded blast radius.
4. **Build Integrity Objective**: reproducible, attestable, byte-identical builds from pinned environments.
5. **Operational Robustness Objective**: PAL/runtime faults cannot silently perturb Domain A state transitions.

### Evidence Taxonomy (what counts as certification evidence)

- **Normative requirements**: `spec/pdf/`, `docs/errata/`, `docs/adr/`.
- **Traceability contract**: `docs/traceability.md` mapping requirement → code → test/vector → proof.
- **Formal evidence**: `proofs/` (compiled Coq objects + hash manifests).
- **Executable semantic evidence**: `model/` extracted/checked observations.
- **Implementation evidence**: `crates/consensus`, `crates/pal`.
- **Dynamic verification evidence**: unit/property tests, cross-ISA replay runs, fuzz campaigns, adversarial scenarios.
- **Supply-chain evidence**: toolchain pinning, container pinning, release attestations, checksums.

### Type and Determinism Control Surface

Certification posture requires explicit type-discipline controls:

- Domain A integer widths are explicit fixed-size types only (`u32/u64/i64/i128`) and never platform-dependent widths.
- No floating point in Domain A.
- No nondeterministic containers or iteration semantics in Domain A.
- Overflow behavior is protocol-defined and must route to absorbing-halt semantics.
- Canonical encoding is protocol identity and must remain unique and deterministic.

All new code paths in consensus should be reviewed for determinism hazards at PR time against these controls.

---

## Detailed Forward Plan

### Phase 1 — Pre-Genesis Assurance Closure (blocking for lock)

#### P1.1 Formal obligation closure

- Complete deferred proof tracks listed in `proofs/COVERAGE.md` (including clearly deferred cryptographic reductions).
- Keep explicit separation between:
  - proved theorem rows,
  - CI-verified behavioral claims,
  - axiom rows with justification,
  - placeholders/deferred reductions.
- For each closure, add/update:
  1. theorem statement location,
  2. implementation binding path,
  3. test/vector witness,
  4. CI artifact hash provenance.

#### P1.2 Domain A deep audits (module hardening)

Audit targets:
- `crates/consensus/src/fixed_point.rs`
- `crates/consensus/src/encoding.rs`
- `crates/consensus/src/lyapunov.rs`
- `crates/consensus/src/hash.rs` and cascade interfaces
- `crates/consensus/src/transaction.rs`

Audit method:
- static invariant checklist,
- malformed/adversarial boundary enumeration,
- panic-path elimination confirmation,
- deterministic error-semantic verification,
- test/proof traceability deltas filed in `docs/traceability.md`.

#### P1.3 Fuzz and adversarial closure

Maintain and continuously run fuzz targets from `fuzz/fuzz_targets/`:
- encoding/decode robustness,
- transition invariants,
- fixed-point boundary arithmetic,
- lyapunov threshold behavior,
- transaction admissibility,
- cascade/hash input structure.

For certification-grade use, archive campaign metadata:
- corpus seed ID,
- execution budget,
- crash reproducer status,
- toolchain/container digest,
- commit hash,
- result summary.

#### P1.4 Genesis lock readiness gate

Before lock, require all of:
- normative PDF committed under `spec/pdf/`,
- traceability links reconciled,
- unresolved errata/ADR blockers resolved or explicitly accepted,
- cross-ISA replay vectors green,
- proof matrix state frozen with artifact hashes.

---

### Phase 2 — Operational Hardening (deployment precondition)

#### P2.1 PAL host implementation controls

Implement real runtime components while preserving Domain A isolation:
- transport subsystem (deterministic framing at Domain A boundary),
- persistent storage with crash-safe WAL semantics,
- replay-from-genesis recovery checks,
- explicit failure-mode handling (network partitions, partial writes, process restarts).

#### P2.2 Integration and fault-injection testing

Add end-to-end suites that validate Domain B does not perturb Domain A semantics:
- replay equivalence under restart and crash-recovery paths,
- network delay/reorder/drop adversarial scenarios,
- deterministic convergence checks under node churn,
- durable state/root consistency across restarts.

#### P2.3 Performance characterization with safety margins

Measure and archive:
- worst-case epoch transition time,
- peak stack/heap footprints by path,
- serialization throughput and latency distribution,
- replay throughput under high-divergence workloads.

Include acceptance envelopes and regression thresholds in CI gates.

---

### Phase 3 — Assurance Hardening (independent auditability)

#### P3.1 Proof-to-code refinement

Strengthen formal linkage between `proofs/model/Model.v` and `crates/consensus`:
- declare refinement surface explicitly,
- prove observational equivalence for scoped transitions/encodings,
- version and pin proof/object outputs.

#### P3.2 Multi-compiler and backend differential verification

Execute deterministic replay corpus across multiple compiler/backend configurations (e.g., LLVM variants and additional backend oracle where practical), and fail on root divergence.

#### P3.3 Trusted computing base minimization

Track and reduce trust assumptions via:
- documented axiom minimization,
- explicit assumption ledger updates,
- independent reproduction of build/proof artifacts.

---

## CI/CD Architecture for High-Assurance Operation

### Required CI lanes

1. **Style and lint lane**
   - formatting and static lint checks for all Rust/Coq/docs pipelines.
2. **Consensus correctness lane**
   - `cargo test --no-default-features` and deterministic invariants.
3. **Cross-ISA replay lane**
   - replay corpus execution on x86_64/aarch64/riscv64gc (QEMU where required).
4. **Proof lane**
   - Coq compile, admitted-marker rejection, axiom-coverage check, proof-object hashing.
5. **Fuzz smoke lane**
   - bounded deterministic-budget campaign on all registered fuzz targets.
6. **Adversarial scenario lane**
   - curated halt-trigger/liveness/replay attack simulations.
7. **Reproducible build + attestation lane**
   - two-stage byte-identical build comparison, artifact checksum publication.

### CI gating policy

- Any consensus-affecting PR must pass correctness + replay + proof lanes.
- Any encoding/state-root-affecting PR must additionally pass vector parity and cross-ISA replay lanes.
- Any cryptographic surface PR must include KAT updates and assumption/proof impact notes.

---

## Certification/Accreditation Readiness Package (deliverables)

For external certification-grade review, maintain a release bundle with:

1. **Requirements baseline** (normative PDF hash + errata/ADR set).
2. **Traceability matrix snapshot** (`docs/traceability.md` frozen with commit hash).
3. **Proof object manifest** (Coq version + `.vo` hash index + build environment digest).
4. **Cross-ISA replay evidence** (vector corpus ID, outputs, platform manifests).
5. **Fuzz/adversarial evidence** (campaign metadata + reproducible rerun commands).
6. **Reproducible build attestation** (byte-equality proofs and signed checksums).
7. **Risk register** (open assumptions, deferred obligations, compensating controls).

A release should not be marked certification-ready unless all seven are present and mutually consistent at the same commit boundary.

---

## Immediate Execution Plan (next implementation sequence)

1. Finish Phase 1 deep module audits and convert all findings into traceable issues/PRs.
2. Close or explicitly defer remaining proof obligations with documented acceptance criteria.
3. Freeze genesis-lock evidence set and run full cross-ISA + proof + fuzz + adversarial CI matrix.
4. Build Phase 2 PAL host with strict Domain A boundary tests and crash-recovery equivalence checks.
5. Advance Phase 3 refinement and multi-compiler differential evidence for independent assurance.

---

## Roadmap Gap Closure (v1.1 Features vs Kernel-Reduced Architecture)

The repository tracks two complementary planning views:

- **Feature migration view** (v1.0→v1.1): deterministic ordering, epoch-seed/timing controls, cascade/health mechanics, migration compatibility windows, and cross-ISA replay validation.
- **Kernel-reduced architecture view**: semantic-kernel closure, compile-time domain enforcement, hardened Domain B profiles, privacy/compliance normalization, and certification-evidence packaging.

Both are required for high-assurance delivery:
- Feature migration explains what changed.
- Kernel roadmap explains why the trusted core is auditable and certifiable.

See `ARCHITECTURE.md` for the full phased architecture roadmap and CI/evidence gating model.

This sequence is intentionally conservative: no deployment claims should be made before Phase 2 closure, and no high-assurance certification claim should be made before substantive Phase 3 evidence is complete.
