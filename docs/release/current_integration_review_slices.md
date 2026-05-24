# Current Integration Review Slices

**Date:** 2026-05-21
**Scope:** Pre-genesis integration RC, not a genesis-lock decision.

Review the current integration branch as the following logical slices. Keep the
slices separate in PRs or commits when possible; if they remain in one branch,
use this file as the review map.

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

## Canonical Vector Runner Path

- Canonical runner location: `crates/vector-runner/` (`qash-vector-runner`).
- Purpose split:
  - **CI gate:** replay determinism is enforced by `qash-consensus` replay tests (`v1_1_replay`, `v1_2_sharded_replay`) in CI.
  - **Local diagnostics:** `cargo run -p qash-vector-runner -- --vectors tests/vectors/vectors.v1.json --out out.native.json` for emitting and checking executable vector outputs.
- `tests/vector-runner/` is deprecated and removed so a single workspace dependency graph governs replay-vector execution logic.
