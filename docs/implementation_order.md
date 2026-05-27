# QASH Implementation Order

**Status:** Active execution guide.  
**Last updated:** 2026-05-27 (Phase 5 hardening complete — all PRs #174–#184 merged)  
**Purpose:** This file records completed pre-genesis RC work and orders the remaining evidence/genesis-lock work. `ROADMAP.md` describes the broader destination.

## Current posture

QASH is pre-genesis RC. Domain A (24 modules, 483+ tests passing) is implementation-complete
for the current RC surface, but not production-authoritative until evidence reconciliation,
tracked proof-debt review, cross-ISA evidence, compliance evidence, and genesis-lock gates are complete.

Domain B PAL is substantially complete: production TCP transport, crash-recovery parity
harness, Plonky3 ZK verifier scaffold, privacy/erasure, and FIPS-aligned crypto are all
implemented and merged.

Active checked Coq proof files are clean of `Admitted.` markers. Remaining proof debt is
explicitly tracked as axioms/placeholders in `proofs/COVERAGE.md`; do not describe the proof
system as fully discharged until those tracked assumptions are closed or accepted by an owner
sign-off for the relevant release boundary.

**Release baselines (as of 2026-05-27):**
- `qash-pilot-baseline-v0.2.1` = commit `04ad39d` (Merge PR #168)
- `qash-pilot-baseline-v0.3` = commit `67665e4` (Merge PR #169)
- `qash-phase5-complete` = current `main` (all PRs #174–#184 merged, 2026-05-27)
- Genesis lock tag is deferred until the evidence gate below is complete

**Already merged (do not re-implement):**
- PR #75: v1.1 CI toolchain stabilization (cross-compile matrix)
- PR #77: v1.1 Envelope primitives and causal ordering
- PR #167: Phase 2-R micro-fix — single-pass tx admission via cheap byte reads (partial 2-R landing)
- PR #168: Pilot execution readiness docs, evidence manifest, pilot package, funding docs
- PR #169: v0.3 multi-operator import/replay with labelled import tracking

**Phase 5 hardening — merged (2026-05-27):**
- PR #174 (Track 3): Privacy/erasure boundary — `ReceiptKey` ZeroizeOnDrop, `ShredKeyEvidence`, PII boundary assertions
- PR #175 (Track 4): Production PAL transport — `TcpCommitmentTransport`, `FaultyTransport`, crash-recovery parity harness
- PR #176 (Track 6): FIPS-aligned crypto — HMAC-DRBG wording audit, TLS validation, `log_pseudonym`, crypto-agility traits
- PR #177 (Track 7): Phase 2-R sort-order determinism test and ADR evidence
- PR #178 (Track 8): Plonky3 FRI-STARK ZK verifier scaffold with profile-lock invariant
- PR #179 (Track 10): Compliance evidence bundle — DPIA, CC EAL4+ ST, OSCAL assessment, reproducible build script
- PR #180 (Track -1/docs): ROADMAP Phase 5 summary update
- PR #181 (Track 9/1-A): CapToken schema Coq proof — 10 theorems, 0 `Admitted.` markers
- PR #182 (Track 9/housekeeping): Remove stale `encoding_injectivity.v` from `_CoqProject`
- PR #183 (Track 7): Streaming state-root parity tests + `ProjectedView` (~80 KB copy elimination)
- PR #184 (Track 7): `prevalidate_all_impl` removes the extra ~80 KB `EpochState` copy before tx prevalidation

Genesis remains trustless and deterministic. Hardware-backed tools are optional local OpSec only.

## Strategic order

1. **Consolidate gates before new protocol work** (Track 0 — pre-execution gate).
   - Run `bash scripts/check_document_hygiene.sh`, `bash scripts/check_privacy_admission.sh`,
     `bash scripts/check_slice_evidence_freshness.sh`, `cargo test --workspace --no-default-features`,
     `make -C proofs`, `cargo deny check`, `bash scripts/verify_two_stage_build.sh`.
   - Keep the trustless-genesis / vendor-agnostic hardware OpSec invariant green.
   - Keep privacy admission lint for TX-2+ specs green.
   - Keep slice evidence freshness manifests for review-critical work green.
   - Treat PR #93 follow-through as scheduled roadmap work, not current implementation.

2. **Security and compliance preflight — complete for the current RC phase.**
   - CodeQL Rust analysis.
   - OSV dependency scanning.
   - OpenSSF Scorecard.
   - CycloneDX SBOM generation.
   - Secret scanning.
   - Rust hygiene checks.
   - QASH-specific Domain A and hardware-OpSec tripwires.
   - Advisory jobs remain advisory until their findings are triaged and repository-level controls are configured.

3. **Zero-persistence code boundary — complete for the current RC surface.**
   - Production admission consumes `EphemeralEnvelope` by value.
   - Borrowed parser views are used at the PAL boundary.
   - Only validated scalar effects or commitments pass into Domain A.
   - Raw fixture WALs remain under replay/pilot scaffolding boundaries only.

4. **Privacy admission and receipt/key shredding — complete for the current RC surface.**
   - Receipt/key shredding evidence exists.
   - Disclosure-domain declarations exist.
   - Public-transcript no-PII/no-graph-field tests exist.
   - Claims remain "GDPR-aligned design" and "erasure-compatible handling," not "GDPR compliant."

5. **Production PAL transport and recovery — complete for the current RC surface.**
   - Commitment-frame TCP transport exists.
   - Fault-injection transport exists for deterministic reorder/drop tests.
   - Crash-recovery parity harness exists.
   - Attestation remains Domain B local evidence only.

6. **Production ZK verifier backend in Domain B — scaffold merged.**
   - Proof bytes remain out of Domain A.
   - Malformed-proof rejection and profile-lock tests exist.
   - Throughput/finality claims still require prover-sizing evidence and production follow-through.

7. **Phase 2-R runtime optimization — complete for the current RC surface.**
   - PR #167: single-pass admission via cheap byte reads.
   - PR #177: deterministic total-order sorting evidence via reversed-input parity test.
   - PR #183: streaming state-root commitment with exact preimage parity and runtime-only `ProjectedView`.
   - PR #184: `prevalidate_all_impl` removes the extra hot-path `EpochState` copy while keeping public API and wire format unchanged.
   - Accept no performance claim without archived benchmark artifacts under `artifacts/benchmarks/`.

8. **Compliance evidence bundles — design-phase complete; final candidate evidence still required.**
   - SBOM.
   - dependency and vulnerability scans.
   - proof hashes.
   - release attestation.
   - cross-ISA roots.
   - fuzz and Kani summaries.
   - zero-persistence summary.
   - OSCAL-style assessment output.
   - Capture a fresh candidate-commit evidence bundle before any genesis-lock tag.

9. **Active proof posture — admitted-clean, not assumption-free.**
   - `make -C proofs` rejects active `Admitted.` markers.
   - `proofs/COVERAGE.md` reports 42 `PROVED`, 4 `CI-VERIFIED`, 3 `AXIOM`, 2 `PLACEHOLDER`, 0 `MISSING`, 44 total. This file is the source of truth for proof counts; README and roadmap summaries must mirror it mechanically.
   - Treat AX2 refinement, cascade collision resistance, blinding PRF, and IT-MAC proof debt as explicitly tracked assumptions/placeholders until discharged or accepted at a release boundary.

10. **Schedule PR #93 research and production follow-through without blocking genesis.**
   - `ADR-007`: UC-MJA cascade remains research only until KATs, benchmarks,
     constant-time evidence, cross-ISA parity, and proof obligations exist.
   - `ADR-008`: sovereign storage tiers remain Domain B deployment policy;
     raw PII, unencrypted receipts, and keys never go to public/decentralized storage.
   - `ADR-009`: LEANN/vector indexing remains Domain B-only; it cannot drive
     consensus, sharding, finality, or public transcript semantics.
   - Phase 6 sharding/horizontal scaling is post-genesis production delivery,
     not a pre-genesis lock dependency unless explicitly promoted by a future decision.

11. **Make genesis-lock decision only after evidence reconciliation.**
   - Normative PDF committed and reconciled.
   - Traceability complete.
   - Cross-ISA replay evidence current.
   - Production PAL readiness explicit.
   - Compliance/evidence bundle captured for the candidate commit.
   - Active proof files admitted-clean and tracked axioms/placeholders explicitly accepted or discharged for the chosen release boundary.

## Minimum local evidence command set

```bash
bash scripts/check_document_hygiene.sh
bash scripts/check_privacy_admission.sh
git diff --check
cargo fmt --all -- --check
cargo test --workspace --no-default-features
cargo test -p qash-pal --features std
make -C proofs
cargo deny check
scripts/run_kani_consensus.sh
```

---

## Track 11 — Pre-Genesis Full-Repo Audit Gate + Authorised Platform Matrix

**Status:** In progress (PRs #185–#197).  
**Gate:** Must complete before genesis-lock. All blocking audit phases must be clean.

### Deliverables

| PR | Content | Branch |
|----|---------|--------|
| [#185](https://github.com/corp0ratestarts-cmd/qash/pull/185) | `docs/platforms/authorized_platform_matrix.md` | `claude/relaxed-hopper-OHZQZ` |
| [#186](https://github.com/corp0ratestarts-cmd/qash/pull/186) | `docs/platforms/rtos_portability_plan.md` | `docs/rtos-portability-plan` |
| [#187](https://github.com/corp0ratestarts-cmd/qash/pull/187) | `docs/platforms/accelerator_profiles.md` | `docs/accelerator-profiles` |
| [#188](https://github.com/corp0ratestarts-cmd/qash/pull/188) | `docs/audit/pre_genesis_audit_plan.md`, `docs/audit/unsafe_exceptions.md`, `artifacts/audit/.gitkeep` | `docs/pre-genesis-audit-plan` |
| [#189](https://github.com/corp0ratestarts-cmd/qash/pull/189) | `scripts/audit_file_inventory.sh`, `scripts/audit_claim_boundary.sh` | `scripts/audit-file-inventory-claim-boundary` |
| [#190](https://github.com/corp0ratestarts-cmd/qash/pull/190) | `scripts/audit_rust_bad_practices.sh`, `scripts/audit_panic_surface.sh` | `scripts/audit-rust-bad-practices-panic-surface` |
| [#191](https://github.com/corp0ratestarts-cmd/qash/pull/191) | `scripts/audit_unsafe_boundary.sh`, `scripts/audit_liveness_loops.sh`, `scripts/audit_concurrency_patterns.sh`, `scripts/audit_domain_boundary_full.sh` | `scripts/audit-e-boundary-liveness-concurrency` |
| [#192](https://github.com/corp0ratestarts-cmd/qash/pull/192) | `scripts/run_platform_determinism_gate.sh` | `scripts/platform-determinism-gate` |
| [#193](https://github.com/corp0ratestarts-cmd/qash/pull/193) | `docs/audit/dependency_risk_register.md` | `docs/audit-f-dependency-risk-register` |
| [#194](https://github.com/corp0ratestarts-cmd/qash/pull/194) | `scripts/run_clippy_strict_audit.sh`, `.github/workflows/pre-genesis-full-repo-audit.yml` | `scripts/audit-g-clippy-workflow` |
| [#195](https://github.com/corp0ratestarts-cmd/qash/pull/195) | `scripts/build_pre_genesis_audit_report.sh` | `scripts/audit-h-report-builder` |
| [#196](https://github.com/corp0ratestarts-cmd/qash/pull/196) | `scripts/build_platform_evidence_matrix.sh`, `artifacts/evidence/platform_matrix.template.md` | `scripts/platform-evidence-matrix` |
| [#197](https://github.com/corp0ratestarts-cmd/qash/pull/197) | `.github/workflows/platform-determinism-advisory.yml`, `os-determinism-advisory.yml`, `embedded-nostd-advisory.yml` | `workflows/platform-advisory` |
| [#198](https://github.com/corp0ratestarts-cmd/qash/pull/198) | Link audit/platform docs from README, ROADMAP, this file | `docs/pr-j-link-audit-platform-docs` |

### Audit phases

| Phase | Script | Status | Gate |
|-------|--------|--------|------|
| 1 — File inventory | `audit_file_inventory.sh` | ✅ Done | Full audit only |
| 2 — Rust bad practices | `audit_rust_bad_practices.sh` | ✅ Done | Blocking |
| 3 — Strict Clippy | `run_clippy_strict_audit.sh` | ✅ Done | Advisory |
| 4 — Unsafe boundary | `audit_unsafe_boundary.sh` | ✅ Done | Blocking |
| 5 — Liveness loops | `audit_liveness_loops.sh` | ✅ Done | Blocking |
| 6 — Panic surface | `audit_panic_surface.sh` | ✅ Done | Blocking |
| 7 — Concurrency patterns | `audit_concurrency_patterns.sh` | ✅ Done | Advisory |
| 9 — Claim boundary | `audit_claim_boundary.sh` | ✅ Done | Blocking |
| 10 — Domain A/B full boundary | `audit_domain_boundary_full.sh` | ✅ Done | Blocking |
| 12 — Consolidated report | `build_pre_genesis_audit_report.sh` | ✅ Done | Full audit only |

### Platform tiers

| Tier | Scope | CI status |
|------|-------|-----------|
| **A** — genesis-blocking | `x86_64`, `aarch64`, `riscv64gc` Linux | ✅ `platform-determinism.yml` |
| **A+** — advisory ISA | musl, i686, s390x, LoongArch, ARMv7 | `platform-determinism-advisory.yml` |
| **B** — hosted OS | Windows, macOS, FreeBSD, WASM | `os-determinism-advisory.yml` |
| **C** — embedded / RTOS | thumbv7em, thumbv8m, riscv32imac, aarch64-none | `embedded-nostd-advisory.yml` |
| **D** — accelerator / hardware | MUSA, CUDA, ROCm, TPM, HSM, TEE | Domain B evidence profiles (manual) |

### Policy documents

- [`docs/platforms/authorized_platform_matrix.md`](../platforms/authorized_platform_matrix.md) — Evidence-gating rules and tier definitions
- [`docs/platforms/rtos_portability_plan.md`](../platforms/rtos_portability_plan.md) — RTOS portability strategy
- [`docs/platforms/accelerator_profiles.md`](../platforms/accelerator_profiles.md) — Accelerator and hardware evidence profiles
- [`docs/audit/pre_genesis_audit_plan.md`](pre_genesis_audit_plan.md) — Full audit plan with all phase specs
- [`docs/audit/unsafe_exceptions.md`](unsafe_exceptions.md) — Unsafe exception register
- [`docs/audit/dependency_risk_register.md`](dependency_risk_register.md) — Dependency risk triage register
