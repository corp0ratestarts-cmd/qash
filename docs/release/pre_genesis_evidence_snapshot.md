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

Phase 1 prerequisite status (updated 2026-05-30):

| Item | Status |
|------|--------|
| 1-A: Reconcile genesis schema v1.0/v1.1 weights | ✅ Done — two-section approach in `spec/genesis.schema.toml` |
| 1-B: Fix stale `02_test_vectors.md` reference | ✅ Done — `docs/spec/02_transition_axioms.md` updated |
| 1-C: ADR-003 full byte-layout spec | ✅ Done — normative spec in `docs/adr/ADR-003-state-root-and-encoding.md`; PDF-golden pending 1-D |
| 1-D: Manual PDF traceability verification | ❌ Human task — not automatable |
| 1-E: Consolidate duplicate ADR variants | ✅ Done — SUPERSEDED status in ADR-001/ADR-003 variants; README rewritten |
| 1-F: Proof-debt classification | ✅ Done — `proofs/COVERAGE.md` has v1.0 genesis-lock proof-debt section |

Remaining genesis-lock blocker: **Phase 1-D** (human PDF review) and then **Phase 1-G** (lock commit + v1.0-reference tag).

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

## Phase 2–6 CI and Evidence Hardening (completed 2026-05-30)

The following CI/evidence work is complete on branch `claude/modest-gates-tgIDP`:

| Phase | Item | Status |
|-------|------|--------|
| 2-A | Domain A tripwires (f32/f64, HashMap, usize struct fields) | ✅ |
| 2-B | Cranelift differential — blocking, pinned nightly | ✅ |
| 2-C | Miri-consensus blocking job | ✅ |
| 2-D | `boundary_fuzz` target (CommitmentFrame roundtrip) | ✅ |
| 2-E | Golden vector hash lock (`vector-integrity` CI job) | ✅ |
| 2-F | Bench-gate advisory job with `benchmarks/baseline.json` | ✅ |
| 2-G | OSV scan promoted to blocking | ✅ |
| 3-A | Blinding PRF proof — Coq theorems proved, COVERAGE.md updated | ✅ |
| 3-B | IT-MAC forgery bound proof — Coq theorems proved, COVERAGE.md updated | ✅ |
| 3-E | `proofs/REFINEMENT.md` — three-layer chain documentation | ✅ |
| 4-A | Hardware attestation backend stubs (TPM2, TDX, SEV-SNP, ARM-CCA) | ✅ scaffold |
| 5-A | `docs/compliance/internal_crypto_evidence_matrix.md` | ✅ |
| 5-B | `docs/compliance/kat_policy.md` — KAT coverage policy | ✅ |
| 5-C | `docs/security/SECURITY_POLICY.md` | ✅ |
| 6-A | `docs/GLOSSARY.md` (in genesis artifacts) | ✅ |
| 6-B | `docs/adr/README.md` — complete ADR index | ✅ |
| 6-C | Fix `02_test_vectors.md` reference | ✅ |
| 6-D | `docs/security/HAZOP.md` | ✅ |
| 6-E | `.github/workflows/fuzz-extended.yml` — weekly 1M execs | ✅ |

## Next Execution Tracks

1. Perform Phase 1-D: manual PDF traceability verification against `spec/pdf/QASH_Spec_v1.0.pdf`.
2. Once 1-D is complete: run `cargo run --bin genesis-hash`, update `GENESIS_CONSTANTS.toml`,
   flip `genesis_status = "locked"` and `deployment_authoritative = true`, tag `v1.0-reference`.
3. Archive final evidence bundle with `bash scripts/capture_pre_genesis_evidence.sh`.
