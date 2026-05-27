# QASH Implementation Order

**Status:** Active execution guide.  
**Last updated:** 2026-05-27 (Phase 5 hardening tracks in review — PRs #174–#183)  
**Purpose:** This file orders the remaining work. `ROADMAP.md` describes the broader destination.

## Current posture

QASH is pre-genesis RC. Domain A (24 modules, 483 tests passing) is implementation-complete
for the current RC surface, but not production-authoritative until proof closure, cross-ISA
evidence, compliance evidence, and genesis-lock gates are complete.

Domain B is the operational boundary for networking, storage, privacy, attestation,
acceleration, audit, and evidence. The PAL scaffold is substantially built; production
transport, ZK verifier, and crash recovery remain open.

**Release baselines (as of 2026-05-26):**
- `qash-pilot-baseline-v0.2.1` = commit `04ad39d` (Merge PR #168)
- `qash-pilot-baseline-v0.3` = commit `67665e4` (Merge PR #169, current `main`)
- Genesis lock tag is deferred until the evidence gate below is complete

**Already merged (do not re-implement):**
- PR #75: v1.1 CI toolchain stabilization (cross-compile matrix)
- PR #77: v1.1 Envelope primitives and causal ordering
- PR #167: Phase 2-R micro-fix — single-pass tx admission via cheap byte reads (partial 2-R landing)
- PR #168: Pilot execution readiness docs, evidence manifest, pilot package, funding docs
- PR #169: v0.3 multi-operator import/replay with labelled import tracking

**Phase 5 hardening — in review (PRs #174–#183, all from decomposed #172):**
- PR #174 (Track 3): Privacy/erasure boundary — `ReceiptKey` ZeroizeOnDrop, `ShredKeyEvidence`, PII boundary assertions
- PR #175 (Track 4): Production PAL transport — `TcpCommitmentTransport`, `FaultyTransport`, crash-recovery parity harness
- PR #176 (Track 6): FIPS-aligned crypto — HMAC-DRBG wording audit, TLS validation, `log_pseudonym`, crypto-agility traits
- PR #177 (Track 7): Phase 2-R sort-order determinism test and ADR evidence
- PR #178 (Track 8): Plonky3 FRI-STARK ZK verifier scaffold with profile-lock invariant
- PR #179 (Track 10): Compliance evidence bundle — DPIA, CC EAL4+ ST, OSCAL assessment, reproducible build script
- PR #180 (Track -1/docs): ROADMAP Phase 5 summary update
- PR #181 (Track 9/1-A): CapToken schema Coq proof — 10 theorems, 0 Admitted
- PR #182 (Track 9/housekeeping): Remove stale `encoding_injectivity.v` from `_CoqProject`
- PR #183 (Track 7): Streaming state-root parity tests + `ProjectedView` (~80 KB copy elimination)

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

2. **Expand CI into security and compliance preflight.**
   - CodeQL Rust analysis.
   - OSV dependency scanning.
   - OpenSSF Scorecard.
   - CycloneDX SBOM generation.
   - Secret scanning.
   - Rust hygiene checks.
   - QASH-specific Domain A and hardware-OpSec tripwires.

3. **Implement zero-persistence as code.**
   - Split PAL features into `replay-scaffold`, `zero-persistence`, and
     `sovereign-hardened`.
   - Make production admission consume `EphemeralEnvelope` by value.
   - Use borrowed parser views only.
   - Pass only validated scalar effects or commitments into Domain A.
   - Keep raw fixture WALs only under `replay-scaffold`.

4. **Implement privacy admission and receipt/key shredding.**
   - Add receipt encryption declarations.
   - Add disclosure-domain declarations.
   - Add `ShredCommitment` evidence.
   - Add public-transcript no-graph-field tests.

5. **Implement production PAL transport and recovery.**
   - Commitment-frame transport.
   - Crash-safe commitment WAL.
   - Replay-from-genesis recovery.
   - Network reorder/drop/delay tests.
   - Attestation as Domain B local evidence only.

6. **Implement production ZK verifier backend in Domain B.**
   - Keep proof bytes out of Domain A.
   - Add malformed-proof rejection and profile-lock tests.
   - Preserve v1.2 sharded replay parity.
   - Produce prover-sizing evidence before throughput or finality claims.

7. **Complete Phase 2-R runtime optimization (partial — PR #167 landed single-pass admission).**
   - Archive tx-heavy and commit-path benchmark baselines first (`cargo criterion`; 1024-validator epoch).
   - Confirm PR #167 parity on all 3 ISAs (x86_64, aarch64, riscv64gc), not just x86_64.
   - Implement deterministic total-order sorting by `(sort_key, tx_id)`; add reversed-input parity test.
   - Implement streaming state-root commitment with exact preimage parity against current buffered path.
   - Implement runtime-only `ProjectedView`; keep Coq model unchanged.
   - Write ADR evidence in `docs/adr/ADR-006-runtime-optimization-track.md`.
   - Accept no performance claim without archived benchmark artifacts under `artifacts/benchmarks/`.

8. **Build per-commit compliance evidence bundles.**
   - SBOM.
   - dependency and vulnerability scans.
   - proof hashes.
   - release attestation.
   - cross-ISA roots.
   - fuzz and Kani summaries.
   - zero-persistence summary.
   - OSCAL-style assessment output.

9. **Schedule PR #93 research and production follow-through without blocking genesis.**
   - `ADR-007`: UC-MJA cascade remains research only until KATs, benchmarks,
     constant-time evidence, cross-ISA parity, and proof obligations exist.
   - `ADR-008`: sovereign storage tiers remain Domain B deployment policy;
     raw PII, unencrypted receipts, and keys never go to public/decentralized storage.
   - `ADR-009`: LEANN/vector indexing remains Domain B-only; it cannot drive
     consensus, sharding, finality, or public transcript semantics.
   - Phase 6 sharding/horizontal scaling is post-genesis production delivery,
     not a pre-genesis lock dependency unless explicitly promoted by a future decision.

10. **Make genesis-lock decision only after evidence reconciliation.**
   - Normative PDF committed and reconciled.
   - Traceability complete.
   - Cross-ISA replay evidence current.
   - Production PAL readiness explicit.
   - Compliance/evidence bundle captured for the candidate commit.

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
