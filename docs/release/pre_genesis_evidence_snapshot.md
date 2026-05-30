# Pre-Genesis Evidence Snapshot

**Date:** 2026-05-30
**Status:** Post-GRC evidence snapshot. Genesis remains provisional; no genesis-lock decision yet.

This snapshot records the evidence shape after the GRC-7-7-v2 gate completed.
It is intentionally narrower than a genesis-lock release candidate: it documents
what is verified now and what remains as a prerequisite for Phase 1 (genesis lock).

## Current Repository State

As of 2026-05-30, the GRC and cascade hardening work is complete on `main`:

- PR #213 landed QASH-CASCADE-7 cascade hardening (LSH-512, Kupyna-512, full L1 primitive suite).
- PR #214 landed GRC-7-7-v2 certificate generator (`src/bin/genesis_cert.rs`), Argon2id
  work-factor gate, and GRC certificate values in `GENESIS_CONSTANTS.toml`.
- PR #215 replaced the weak GRC parity assertion with a real preimage fixture test
  (`tests/genesis_preimage_parity.rs`).

Genesis status: `provisional`, `deployment_authoritative = false`. The genesis hash
diverges from the recorded value as expected (provisional state); `verify_genesis_hash.sh`
exits 0 with a notice.

Remaining phase-1 prerequisites before any genesis lock:
- 1-A: Reconcile `spec/genesis.schema.toml` vs `GENESIS_CONSTANTS.toml` (v1.0 / v1.1 weight sections)
- 1-B: Fix stale `02_test_vectors.md` cross-reference in `docs/spec/02_transition_axioms.md`
- 1-C: Complete ADR-003 byte-layout spec; lock golden vectors as PDF-golden
- 1-D: Manual PDF traceability verification (human task)
- 1-E: Consolidate duplicate ADR-001 / ADR-003 variants
- 1-F: Classify proof-debt by active claim boundary

The current post-GRC local verification on `main` (2026-05-30) passed:

```bash
git diff --check
cargo fmt --all -- --check
cargo test --workspace --no-default-features
cargo clippy --workspace -- -D warnings
bash scripts/verify_genesis_hash.sh   # exits 0 with provisional notice
```

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
- Domain B hardware/offline stubs are present as deterministic, software-backed
  scaffolds for pre-genesis review; they are not production hardware-backed
  attestation or deployment claims.

## Claims Not Yet Allowed

The following claims require future evidence:

- Genesis lock or deployment authority.
- Production networking, hardware attestation, or crash-recovery hardening.
- Production Plonky3 verification.
- Sub-50ms finality or tx-heavy performance claims without archived benchmark
  artifacts.
- Genesis-lock reconciliation before `spec/pdf/QASH_Spec_v1.0.pdf` is committed
  and traceability is verified against it.

## Next Execution Tracks

1. Keep `main` green while waiting for the normative PDF artifact tracked by
   issue #209.
2. Archive proof, replay, PAL, Kani, and document-hygiene evidence for the exact
   reviewed commit with `bash scripts/capture_pre_genesis_evidence.sh`.
3. Keep production PAL/ZK backend work separate from pre-genesis scaffold work.
4. Reconcile normative PDF, traceability, genesis hash, and release sign-off
   before any lock/reference tag decision.
