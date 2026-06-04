# Current Integration Review Slices

**Date:** 2026-06-03 (updated: QASH-0 profile boundary complete; QASH-2 evidence pass)
**Scope:** Post-GRC, post-QASH-0, pre-genesis. Not a genesis-lock decision.

**Current state:** PR #237 (QASH-0 profile boundary enforcement) is merged to `main`.
The umbrella repo no longer carries the Pure QASH implementation subtree; it is
replaced by a pointer README and enforced by `scripts/check_profile_boundary.sh`.
This file records the remaining review surface before Phase 1 genesis-lock prerequisites
can be discharged. It is not a release approval and not a genesis-lock decision.

## QASH-0 Profile Boundary — COMPLETE ✅

Completed in PR #237. Status:
- `pure-qash/` implementation subtree removed; replaced with pointer README
- `scripts/check_profile_boundary.sh` — 5 blocking rules wired into CI `document-hygiene`
- `docs/adr/ADR-015-pure-qash-repository-split.md` — normative split decision
- `docs/spec/19_profile_taxonomy.md` — three-profile taxonomy (Pure Core / Regulated / Sovereign)

## QASH-2 Slice Evidence Summary (2026-06-03)

All four integration review slices pass on current `main`:

| Slice | Key check | Result |
|-------|-----------|--------|
| 1: Sharding/EFB | `cargo test -p qash-consensus --test v1_2_sharded_replay` | ✅ PASS |
| 2: PAL scaffold | `cargo test -p qash-pal --features std` | ✅ PASS |
| 2a: Hardware stubs | `bash scripts/check_domain_a_tripwires.sh` | ✅ PASS |
| 3: Proof/refinement | `cargo test -p qash-consensus --test coq_refinement_vectors` | ✅ PASS |
| 4: PR #93 hygiene | `bash scripts/check_document_hygiene.sh` | ✅ PASS |
| 4: Phase 2-R gates | `cargo test -p qash-consensus --test phase2r_preconditions` | ✅ PASS |

---

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

**QASH-2 status:** ✅ Sharded replay corpus matches pinned hash; 1 test pass.

---

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

**QASH-2 status:** ✅ 2 PAL unit tests pass + 1 zero-persistence profile test pass.

---

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

**QASH-2 status:** ✅ Domain A tripwires pass; hardware stubs correctly classified as post-v1.

---

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

**QASH-2 status:** ✅ 3 refinement vector tests pass (encoding vectors, rejection vectors, Lyapunov transition observations).

---

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

**QASH-2 status:** ✅ Document hygiene pass; 6/6 phase2r_preconditions tests pass.

---

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

## Remaining Genesis-Lock Prerequisites

| Item | Status |
|------|--------|
| QASH-0: Profile boundary enforcement | ✅ Done — PR #237 |
| QASH-1: `QASH_Spec_v1.0.pdf` commit | ⏳ BLOCKED — owner must provide PDF |
| QASH-1.2: Record PDF SHA-256 in traceability | ⏳ Blocked on QASH-1 |
| QASH-1.3: Reconcile `docs/traceability.md` against PDF | ⏳ Blocked on QASH-1 |
| QASH-1.4: Reconcile `GENESIS_CONSTANTS.toml` against locked doc set | ⏳ Blocked on QASH-1 |
| QASH-2: Integration review slices (all 4 slices) | ✅ Done — all checks pass |
| Phase 1-G: Final evidence capture + owner sign-off | ⏳ Pending QASH-1 unblock |
