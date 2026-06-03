# QASH Completion Plan

**Scope:** Umbrella QASH only. This plan excludes Pure QASH implementation work except where profile-boundary cleanup is required.

**Current state:** QASH is at an RC-only milestone. `GENESIS_CONSTANTS.toml` remains provisional and `deployment_authoritative = false`. A future genesis-candidate requires a separate owner-gated PR.

## Completion objective

Move the umbrella repository from RC-only to a genesis-candidate-ready state while preserving the profile split:

- QASH umbrella owns regulated profile work, compliance evidence, sovereign-hardened research, deployment research, and post-v1 extensions.
- Pure QASH owns its own implementation, genesis constants, absence guards, and release path.
- QASH v1.0 must not claim production capabilities that remain scaffold, demo-only, or post-v1.

## Phase Q0 — Boundary and repository hygiene

| ID | Work item | Exit criteria |
|----|-----------|---------------|
| Q0.1 | Close stale command/tooling branches | Project command PRs either merged or closed; no duplicate open PRs. |
| Q0.2 | Quarantine or remove implementation subtree not owned by umbrella QASH | Pure implementation artifacts do not contaminate umbrella release artifacts. |
| Q0.3 | Keep profile taxonomy and ADR-015 | Umbrella retains governance documents defining profile boundaries. |
| Q0.4 | Add release-note warning | RC evidence states QASH umbrella and Pure implementation tracks have separate release authority. |

## Phase Q1 — Normative PDF and genesis-lock prerequisites

| ID | Work item | Exit criteria |
|----|-----------|---------------|
| Q1.1 | Commit authoritative `spec/pdf/QASH_Spec_v1.0.pdf` | PDF exists in the expected path and SHA-256 is recorded. |
| Q1.2 | Reconcile `docs/traceability.md` | Every traceability row cites the committed PDF or explicitly records `PDF-SILENT` with ADR link. |
| Q1.3 | Reconcile genesis artifacts | `spec/genesis-artifacts.txt`, `GENESIS_CONSTANTS.toml`, and evidence snapshot agree. |
| Q1.4 | Capture final evidence bundle | `scripts/capture_pre_genesis_evidence.sh` produces a clean manifest for the exact commit. |
| Q1.5 | Owner sign-off PR | Outcome A PR includes `[genesis-change-acknowledged]`. |

## Phase Q2 — Current integration review slices

| Slice | Work item | Required evidence |
|-------|-----------|-------------------|
| S1 | Sharding and EFB scaffold | `cargo test -p qash-consensus --test v1_2_sharded_replay`; vector integrity; proof build. |
| S2 | PAL whole-protocol scaffold | `cargo test -p qash-pal --features std`; Domain B nondeterminism cannot affect Domain A. |
| S2a | Hardware and offline stubs | Stub maturity table; no production hardware-backed attestation claim. |
| S3 | Proof/refinement closure | `make -C proofs`; Coq/Rust vectors; proof status refresh. |
| S4 | Hygiene and Phase 2-R gates | document hygiene; phase2r tests; benchmark no-run; evidence capture. |

## Phase Q3 — Regulated profile track

| ID | Work item | Exit criteria |
|----|-----------|---------------|
| REG.1 | Define regulated feature boundary | Regulated builds cannot claim Pure privacy properties. |
| REG.2 | Define Class IV observer semantics | Observer role, scope, and limitations are normative. |
| REG.3 | Define lawful-basis state machine | Disclosure flow is explicit, auditable, and epoch-scoped. |
| REG.4 | Define disclosure key lifecycle | Genesis authorization, rotation, revocation, and audit logs are specified. |
| REG.5 | Add regulated receipt API | Tests cover permitted and rejected disclosure flows. |
| REG.6 | Add compliance evidence bundle | Evidence shape is jurisdiction-neutral and does not overclaim certification. |

## Phase Q4 — Sovereign Hardened profile track

This track remains post-v1 unless explicitly pulled into the genesis-candidate critical path.

| ID | Work item | Exit criteria |
|----|-----------|---------------|
| SOV.1 | Define sovereign threat model | Hardware assumptions and non-assumptions are explicit. |
| SOV.2 | Define attestation evidence schema | Quote fields, nonce binding, identity binding, and verifier boundaries are specified. |
| SOV.3 | Implement backend registry | TPM2, TDX, SEV-SNP, ARM CCA, HSM backends are classified by maturity. |
| SOV.4 | Add negative tests | Malformed quote and stale quote rejection are tested. |
| SOV.5 | Preserve local-failure semantics | Hardware failure affects local node only; no global liveness dependency. |

## Phase Q5 — Production networking and peer discovery

Production networking remains a not-yet-allowed claim until this phase is complete.

| ID | Work item | Exit criteria |
|----|-----------|---------------|
| NET.1 | Define peer discovery spec | No mandatory single seed host, DNS seed, or centralized bootstrap path. |
| NET.2 | Define deterministic fallback routing | Fallback order is deterministic and auditable. |
| NET.3 | Define partition behavior | EFB quorum behavior under partial isolation is specified. |
| NET.4 | Add simulation tests | Partition, delayed peer, and unreachable peer cases are covered. |
| NET.5 | Add claim guard | Release docs reject production networking claims until evidence exists. |

## Phase Q6 — ZK and threshold signing production gates

Production ZK verification and production threshold signing remain not-yet-allowed claims until this phase is complete.

| ID | Work item | Exit criteria |
|----|-----------|---------------|
| ZK.1 | Define verifier boundary | Interface-only, advisory, and production verifier states are distinct. |
| ZK.2 | Implement or defer production verifier | Release claims match implementation maturity. |
| ZK.3 | Add proof-state distinction | Invalid proof, missing proof, and delayed proof are handled separately. |
| ZK.4 | Add replay fallback | Proof latency does not directly become global liveness failure. |
| THR.1 | Replace placeholder share combine | Production threshold signing no longer uses demo placeholder logic. |
| THR.2 | Add threshold test vectors | Valid and malformed share sets are covered. |
| THR.3 | Define quorum failure behavior | Liveness and safety behavior are explicit. |

## Phase Q7 — Genesis-candidate PR

Only after Q1 and the chosen v1.0 scope of Q2-Q6 are complete:

1. Open an owner-gated PR selecting Outcome A.
2. Include `[genesis-change-acknowledged]` in the PR body.
3. Set `genesis_status = "genesis-candidate"`.
4. Keep `deployment_authoritative = false` unless a separate production deployment sign-off exists.
5. Recompute the genesis hash.
6. Update `spec/genesis-artifacts.txt`.
7. Update `docs/release/pre_genesis_evidence_snapshot.md`.
8. Capture a final clean evidence bundle.
9. Tag `v1.0-reference` only after all gates pass.

## Recommended v1.0 scope

For a tractable genesis-candidate path, QASH v1.0 should lock:

- deterministic Domain A core;
- profile taxonomy and boundary rules;
- compliance and evidence scaffolding;
- regulated-profile specification surface;
- clear post-v1 gates for production networking, production ZK, threshold signing, and sovereign hardware.

Do not claim production networking, production ZK verification, production hardware attestation, or production regulated disclosure until their evidence gates are complete.
