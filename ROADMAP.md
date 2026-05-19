# QASH Roadmap: From Deterministic Consensus to Verified Execution Substrate

This roadmap complements `README.md` and `PROJECT_STATUS.md` and is structured for two audiences:
- **Release audience**: concrete v1.1 feature migration status.
- **Assurance audience**: kernel-reduced, proof-carrying execution substrate path.

## Positioning

QASH is not framed as a conventional blockchain roadmap. The target architecture is a **kernel-reduced verified substrate** where correctness is concentrated into a minimal trusted core and carried through proofs, deterministic replay evidence, and reproducible build attestations.

---

## Track A — v1.1 Feature Migration (Release Notes View)

### A1. Deterministic feature layer
- [x] Deterministic causal ordering via `causal_sort_key`
- [x] Epoch-based timing with skew bounds
- [x] Cascade construction (depth-7) + health tracking
- [x] Weight table revisions aligning Lyapunov evaluation

### A2. Post-quantum and migration layer
- [x] ML-KEM-768 integration path
- [x] 100-epoch compatibility window + conversion path documentation
- [x] Cross-ISA replay verification lane (x86_64/aarch64/riscv64gc)

### A3. Formal baseline
- [x] Active theorem baseline (TH-1..TH-8, TX-0 class evidence split by PROVED/CI-VERIFIED/AXIOM/PLACEHOLDER)
- [x] CI proof lane with admitted-marker rejection + proof object hashing

---

## Track B — Kernel-Reduced Assurance Architecture (Target State)

## Phase 0 — Foundation Stabilization

### Completed
- [x] Pinned Rust toolchain and lockfile discipline
- [x] Reproducible build container and release attestation workflow
- [x] Domain partitioning constraints documented (Domain A vs Domain B)

### Remaining
- [ ] CI hardening: clippy/policy gates that explicitly deny `unsafe`, ambient `std` and unbounded allocation patterns in Domain A scope
- [ ] Explicit compile-scope feature profile for Domain A-only verification builds (`no_std` assurance profile)

## Phase 1 — Semantic Kernel Closure

### 1.1 Effect-capability token sealing
- [ ] Introduce capability-tokenized Domain B effect admission so Domain A can only consume authenticated, schema-bound effect artifacts.
- [ ] Add token verification hooks at transition admission boundaries.
- [ ] Add proof obligations tying token validity to state transition admissibility.

### 1.2 Causal fingerprint coinduction
- [ ] Extend safety relation with causal fingerprint equality, so equivalence is trace-aware, not only terminal-state aware.
- [ ] Add theorem statements showing bisimulation requires fingerprint agreement.

### 1.3 Lyapunov confluence closure
- [ ] Add confluence theorem target for canonical DAG compression classes (Church-Rosser style normal-form uniqueness under admissible reduction orders).
- [ ] Map theorem hooks to runtime normalization pipeline.

### 1.4 Verified interpreter conformance
- [ ] Build property-based differential harness between formal model surface (`proofs/model/Model.v` derived observations) and Rust transition runtime.
- [ ] Gate with large randomized directive corpus and reproducible seeds.

## Phase 2 — Operational Hardening (Domain B)

### 2.1 Runtime infrastructure
- [ ] Network transport implementation with deterministic Domain A boundary framing
- [ ] Durable state persistence and crash-safe WAL/replay recovery
- [ ] Deterministic restart equivalence test suites

### 2.2 Hardware/crypto hardening profile
- [ ] Algorithmic hardening track for post-quantum operations (e.g., masked/duplicated computation profiles where applicable)
- [ ] Memory-fault mitigation profile guidance for hardened deployments
- [ ] Proximity-channel anti-relay design option set for PAL integrations
- [ ] High-assurance key-management profile including threshold-signing options for validator operators

### 2.3 Attestable build extensions
- [ ] Extend attestation lane toward externally verifiable provenance logs and signed build attestations
- [ ] Document deterministic build replay protocol for third-party auditors

## Phase 3 — Privacy and Compliance as Normative Surfaces

### 3.1 Privacy model elevation
- [x] `docs/spec/09_privacy_model.md` exists as technical basis
- [ ] Promote privacy model requirements into explicit release-gating traceability rows (observer classes, transcript constraints, disclosure model)

### 3.2 Public transcript boundary enforcement
- [ ] Formalize/verify typed `PublicTranscript` contract and prohibit non-authorized public-surface expansion without spec+traceability updates

### 3.3 Compliance artifacts
- [ ] Security Target draft scoping the TOE to Domain A consensus kernel
- [ ] DPIA-style privacy mapping artifacts for data-minimization posture
- [ ] Cryptographic validation artifact plan (test-vector/CAVP-style integration strategy)

## Phase 4 — Genesis Lock Preparation

- [ ] Normative PDF commit and lock preconditions closure (`spec/pdf/` authority finalized)
- [ ] Traceability reconciliation across errata/ADR/spec/runtime/proofs
- [ ] Proof matrix freeze + artifact hash index publication
- [ ] Cross-ISA replay + fuzz/adversarial matrix green at freeze commit
- [ ] Recompute and lock genesis hash, then tag genesis release

## Phase 5 — Post-Genesis Economic/Keying Extensions

- [ ] Fixed-supply invariant formalization and enforcement artifacts
- [ ] Receipt-oriented value-transfer privacy surface hardening
- [ ] Blinded fee-selection and anti-front-running mechanism analysis
- [ ] Epoch-bound key-rotation operational profiles
- [ ] Selective disclosure controls for regulated counterparties

---

## CI Lanes Required for the Target State

1. **Determinism/consensus lane**: no-default-feature consensus tests + invariant checks
2. **Cross-ISA replay lane**: x86_64/aarch64/riscv64gc state-root parity
3. **Proof lane**: Coq compile, admitted-marker ban, axiom coverage checks, proof hash artifacts
4. **Fuzz lane**: bounded smoke + scheduled deeper campaigns (seeded, archived)
5. **Adversarial lane**: halt-trigger/liveness/replay stress scenarios
6. **Attestation lane**: reproducible two-stage builds + signed artifact manifests
7. **Compliance evidence lane**: release-bundle completeness checks (traceability/proof/replay/build manifests)

---

## Dual-Publish Recommendation

Maintain two companion documents:
1. **Feature Migration Notes** (short, release-facing): concrete v1.1 changes only.
2. **Architecture Roadmap** (this file, assurance-facing): kernel closure, compliance, certification, and evidence pipeline.

This prevents audience confusion while preserving high-assurance direction.


---

## Gap Analysis: Current State vs Upgraded Design Spec

**Short answer:** current migration content is necessary, but insufficient for the target kernel-reduced, proof-carrying substrate.

### What is already represented (feature migration backbone)

| Item | Status | Notes |
|------|--------|-------|
| Deterministic causal ordering via `causal_sort_key` | ✅ | Total deterministic ordering surface |
| Epoch timing model with skew bounds | ✅ | Logical-time gating and rejection behavior |
| Cascade construction + health tracking | ✅ | Deterministic convergence support |
| ML-KEM-768 integration track | ✅ | Post-quantum KEM migration path |
| Compatibility window + conversion path | ✅ | Controlled transition surface |
| Weight revisions for Lyapunov alignment | ✅ | Stability/threshold calibration |
| Cross-ISA replay CI | ✅ | Replay invariance evidence |
| Coq proof CI baseline | ✅ | Active proof compilation + evidence artifacts |
| Minimal public transcript orientation | ✅ | Public artifact minimization |

### Architectural gaps that must be closed

#### Semantic-kernel closure (frontier risks)
- [ ] Effect-capability token architecture for Domain B→A effect admission
- [ ] Causal fingerprint coinduction in equivalence/safety relations
- [ ] Lyapunov confluence proof target for admissible DAG reductions
- [ ] Verified interpreter conformance lane (formal model ↔ runtime differential testing)

#### Compile-time domain enforcement
- [ ] Domain A compile profile hardening (`no_std`, unsafe-deny, deterministic-only patterns)
- [ ] Type-level purity boundaries (marker traits/capability boundaries)
- [ ] Admission invariant checker hooks on envelope acceptance
- [ ] CI policy checks that reject non-compliant Domain A surfaces

#### Domain B hardware/crypto hardening profile
- [ ] Optional hardened cryptographic execution profiles for fault/SCA resistance
- [ ] Deployment guidance for memory-fault mitigations on commodity hardware
- [ ] Anti-relay controls for proximity channels where PAL uses NFC/BLE-style transports
- [ ] Attestable build provenance extensions and external transparency logging
- [ ] Threshold-signing operational profile for high-assurance validator sets

#### Privacy/compliance normalization
- [ ] Promote privacy model into explicit release gating rows in `docs/traceability.md`
- [ ] Formal/typed `PublicTranscript` enforcement at API boundaries
- [ ] Receipt encryption and selective disclosure operational compliance model

#### Certification artifact pathway
- [ ] Security Target package scoping TOE to Domain A kernel
- [ ] DPIA mapping package for data-minimization and disclosure controls
- [ ] Cryptographic validation artifact workflow (vector/CAVP-style integration plan)
- [ ] Independent reproducible-build verification protocol for third-party auditors

#### Tokonomics/economic module hardening (post-genesis)
- [ ] Fixed-supply invariant evidence package
- [ ] Receipt-based transfer privacy hardening
- [ ] Blinded fee-selection / anti-front-running analysis
- [ ] Epoch-bound key-rotation profile and forward secrecy operations

---

## Critical Decision Policy

The project should explicitly publish both:

1. **Feature Migration Notes** (short, release-facing)  
2. **Architecture Roadmap** (this file, assurance-facing)

This dual-publication policy is mandatory to prevent mixing release messaging with certification-grade architecture closure requirements.

---

## Immediate Closure Actions (Roadmap Alignment)

1. Add semantic-kernel closure tasks as explicit gated deliverables in Phase 1.
2. Add compile-time Domain A enforcement tasks in Phase 0 with CI failure semantics.
3. Add Domain B hardening profile in Phase 2 with deployment-class profiles.
4. Add privacy/compliance deliverables in Phase 3 tied to traceability gates.
5. Add certification artifact release bundle requirements before genesis-lock tagging.
6. Keep Track A release notes concise and separate from assurance architecture closure.
