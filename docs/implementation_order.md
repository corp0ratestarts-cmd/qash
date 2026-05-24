# QASH Implementation Order

**Status:** Active execution guide.  
**Purpose:** This file orders the remaining work. `ROADMAP.md` describes the broader destination.

## Current posture

QASH is pre-genesis. Domain A is the deterministic `no_std` consensus kernel.
Domain B is the operational boundary for networking, storage, privacy, attestation,
acceleration, audit, and evidence. Genesis remains trustless and deterministic.
Hardware-backed tools are optional local OpSec only.

## Strategic order

1. **Consolidate gates before new protocol work.**
   - Keep the trustless-genesis / vendor-agnostic hardware OpSec invariant merged.
   - Land privacy admission lint for TX-2+ specs.
   - Land slice evidence freshness manifests for review-critical work.
   - Normalize PR #93 follow-through into roadmap and this file.

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

7. **Execute Phase 2-R runtime optimization.**
   - Benchmark current baseline first.
   - Implement single-pass admission, deterministic candidate sorting,
     streaming state-root hashing, and runtime-only projected views.
   - Accept no performance claim without archived benchmark artifacts.

8. **Build per-commit compliance evidence bundles.**
   - SBOM.
   - dependency and vulnerability scans.
   - proof hashes.
   - release attestation.
   - cross-ISA roots.
   - fuzz and Kani summaries.
   - zero-persistence summary.
   - OSCAL-style assessment output.

9. **Make genesis-lock decision only after evidence reconciliation.**
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
