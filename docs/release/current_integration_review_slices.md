# Current Integration Review Slices

**Date:** 2026-05-29
**Scope:** Pre-genesis integration RC, not a genesis-lock decision.

**Current state:** The review slices below have been merged to `main` as of
PR #208 and PR #210. This file remains the review map for the merged
pre-genesis RC surface. It is not a release approval and not a genesis-lock
decision.

Review the merged pre-genesis RC surface as the following logical slices. Keep
future changes separated by slice in PRs or commits when possible, and use this
file as the review map.

## Slice 1: Sharding and EFB Scaffold

Primary files:
- `docs/spec/12_sharded_protocol.md`
- `crates/consensus/src/sharding.rs`
- `crates/consensus/tests/v1_2_sharded_replay.rs`
- `tests/vectors/vectors.v1.2.json`
- `proofs/sharding/efb_determinism.v`

Required evidence:
- `cargo test -p qash-consensus --test v1_2_sharded_replay`
- `cargo test -p qash-consensus --test vector_integrity`
- `make -C proofs`

Review focus:
- Shard assignment is deterministic.
- Cross-shard receipt roots and EFB roots are replayable.
- The ZK profile remains a public commitment/profile boundary, not a production verifier claim.

## Slice 2: PAL Whole-Protocol Scaffold

Primary files:
- `crates/pal/src/lib.rs`
- `crates/pal/tests/hosted_replay.rs`
- `crates/pal/tests/whole_protocol.rs`
- `crates/pal/tests/boundary_violations.rs`
- `crates/pal/tests/smartcard.rs`

Required evidence:
- `cargo test -p qash-pal --features std`

Review focus:
- Domain B transport, attestation, and proof-bundle material does not feed nondeterminism into Domain A.
- Hosted replay remains deterministic.
- Production networking, hardware attestation, and Plonky3 verification remain explicitly out of scope.

## Slice 2a: Domain B Hardware and Offline Stubs

Primary files:
- `src/consensus/mod.rs`
- `src/hardware/acceleration.rs`
- `src/hardware/attestation_gate.rs`
- `src/hardware/mod.rs`
- `src/offline/clone.rs`
- `src/offline/mod.rs`

Required evidence:
- `cargo test --lib attestation`
- `cargo test --workspace`
- `cargo fmt --all -- --check`
- `git diff --check`

Review focus:
- Software-backed attestation quotes are exact-shape checked and bind nonce,
  identity, genesis parameter hash, and quote digest material.
- Test nonce and identity fixtures are domain-separated derived values, not
  hard-coded cryptographic literals.
- Hardware/offline code remains Domain B scaffold material and does not create
  a production hardware-backed attestation or deployment claim.

## Slice 3: Proof and Refinement Closure

Primary files:
- `proofs/Makefile`
- `proofs/STATUS.md`
- `proofs/_CoqProject`
- `proofs/composition/th3_system_closure.v`
- `proofs/model/Model.v`
- `proofs/model/transition_observations.json`
- `crates/consensus/tests/coq_refinement_vectors.rs`

Required evidence:
- `make -C proofs`
- `cargo test -p qash-consensus --test coq_refinement_vectors`

Review focus:
- Active Coq files compile with no active `Admitted` outside approved axioms.
- Rust/Coq observation vectors match the executable transition behavior.
- New proof claims are reflected in `proofs/STATUS.md`.

## Slice 4: PR #93 Hygiene and Phase 2-R Gates

Primary files:
- `.github/PULL_REQUEST_TEMPLATE.md`
- `.github/workflows/ci.yml`
- `scripts/check_document_hygiene.sh`
- `scripts/capture_pre_genesis_evidence.sh`
- `docs/adr/ADR-006-runtime-optimization-track.md`
- `docs/release/pre_genesis_evidence_snapshot.md`
- `crates/consensus/tests/phase2r_preconditions.rs`
- `crates/consensus/benches/epoch_transition.rs`

Required evidence:
- `bash scripts/check_document_hygiene.sh`
- `cargo test -p qash-consensus --test phase2r_preconditions`
- `cargo bench -p qash-consensus --no-run`
- `bash scripts/capture_pre_genesis_evidence.sh`

Review focus:
- Raw transcripts and ad hoc root spec files are rejected.
- Phase 2-R has benchmark and parity gates before performance claims.
- Transaction prevalidation has an explicit `(sort_key, tx_id)` total-order
  tie-breaker for equal sort keys.
- Runtime optimization remains consensus-byte-preserving.

## Whole-Branch Evidence

Before requesting final review, capture the full local evidence bundle:

```bash
bash scripts/capture_pre_genesis_evidence.sh
```

Use the newest timestamped `manifest.txt` under `artifacts/evidence/` as the
current passing bundle.

The terminal external blocker for genesis lock is issue #209: commit the
normative `spec/pdf/QASH_Spec_v1.0.pdf`, then reconcile `docs/traceability.md`,
`GENESIS_CONSTANTS.toml`, and release sign-off evidence against that exact PDF.
