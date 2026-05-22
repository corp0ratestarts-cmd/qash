# Pre-Genesis Evidence Snapshot

**Date:** 2026-05-21
**Status:** Draft evidence handoff, not a genesis-lock decision.

This snapshot records the evidence shape for the current pre-genesis integration
RC. It is intentionally narrower than a release candidate: it documents what can
be reviewed now and what remains blocked before any genesis lock or performance
claim.

## Current Review Slices

The current worktree should be reviewed as separate logical slices:

| Slice | Contents | Required evidence |
|-------|----------|-------------------|
| Sharding/EFB scaffold | v1.2 sharding primitives, EFB roots, sharded replay vectors, protocol spec | Consensus replay, v1.2 vector integrity, sharding proof compile |
| PAL whole-protocol scaffold | Hosted replay, commitment transport, attestation interfaces, ZK proof-bundle boundary | `cargo test -p qash-pal --features std` |
| Proof/refinement closure | TH-3 composition closure, model extraction surface, proof status updates | `make -C proofs` from a clean proof build before RC |
| PR #93 hygiene and scheduling | Raw transcript rejection, canonical docs, ADR-006, Phase 2-R scheduling | `bash scripts/check_document_hygiene.sh`, doc review |

The detailed slice map is `docs/release/current_integration_review_slices.md`.

## Local Verification Commands

The following local commands are the minimum evidence set before asking for
review on the current integration branch:

```bash
bash scripts/check_document_hygiene.sh
bash scripts/check_privacy_admission.sh
git diff --check
cargo fmt --all -- --check
cargo test -p qash-consensus --test phase2r_preconditions
cargo bench -p qash-consensus --no-run
cargo test --workspace
cargo test -p qash-pal --features std
make -C proofs
cargo deny check
scripts/run_kani_consensus.sh
```

To capture those checks as a reviewable artifact bundle for the exact worktree,
run:

```bash
bash scripts/capture_pre_genesis_evidence.sh
```

The script writes a timestamped bundle under `artifacts/evidence/` with raw logs,
tool versions, commit identity, working-tree status, and a pass/fail manifest.

Expected caveats:
- `cargo deny check` may emit existing `deny.toml` configuration warnings while
  still reporting advisories, bans, licenses, and sources as OK.
- `scripts/run_kani_consensus.sh` is advisory until the Kani CI install/runtime
  behavior is repeatable.
- `cargo test --workspace` does not exercise PAL `std` integration tests; keep
  the explicit PAL command above.

## Claims Allowed Now

The following claims are supported by current repo artifacts:

- Domain A remains deterministic and `no_std` constrained.
- TX-0 and TX-1 perturbation proof obligations are represented.
- EFB determinism and epoch-bound receipt replay rejection have initial Coq
  coverage.
- Hosted PAL replay and whole-protocol sharded replay are scaffolded.
- PR #93 sharding/ZK design feedback is represented in canonical docs and code.
- Raw transcript and ad hoc root-spec additions are rejected by CI.
- TX-type privacy admission fields for §P5/§P7/§P8 are linted by CI via `bash scripts/check_privacy_admission.sh`.

## Claims Not Yet Allowed

The following claims require future evidence:

- Genesis lock or deployment authority.
- Production networking, hardware attestation, or crash-recovery hardening.
- Production Plonky3 verification.
- Sub-50ms finality or tx-heavy performance claims without archived benchmark
  artifacts.
- Phase 2-R runtime optimization completion.

## Next Execution Tracks

1. Land the current integration branch in reviewable slices.
2. Archive proof, replay, PAL, Kani, and document-hygiene evidence for the exact
   reviewed commit with `bash scripts/capture_pre_genesis_evidence.sh`.
3. Start Phase 2-R only after tx-heavy benchmarks and parity tests exist.
4. Keep production PAL/ZK backend work separate from Phase 2-R.
5. Reconcile normative PDF, traceability, genesis hash, and release sign-off
   before any lock/reference tag decision.
