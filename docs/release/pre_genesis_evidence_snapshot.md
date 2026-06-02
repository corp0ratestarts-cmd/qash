# Pre-Genesis Evidence Snapshot

**Date:** 2026-06-02 (updated: Outcome B RC-only milestone)
**Status:** **Outcome B — RC-only milestone** (PR #228). Owner reviewed the complete
Waves 0–7 evidence suite and selected the RC-only path. `GENESIS_CONSTANTS.toml`
remains `genesis_status = "provisional"`, `deployment_authoritative = false`.
`v1.0-reference` is reserved for a future Outcome A decision.

**RC evidence bundle:** `artifacts/benchmarks/pre_genesis_final.md`
**RC commit:** see `artifacts/benchmarks/pre_genesis_final.md`

This snapshot records the evidence shape at the Outcome B RC milestone. The complete
evidence suite covers: PDF traceability verified (Phase 1-D), receipt encryption
upgraded to ChaCha20-Poly1305 AEAD (XOR stub removed), axiom classification complete,
Coq↔Rust parity at 12 vectors (TV-0..TV-11), Domain B backend boundary documented,
WAL robustness hardened, and benchmark suite showing 1024-validator epoch transition
~312 µs (1440× margin against 450 ms control-loop budget).

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
| 1-D: Manual PDF traceability verification | ✅ Done — PDF-verified 2026-06-01 (Wave 1 PR #227) |
| 1-E: Consolidate duplicate ADR variants | ✅ Done — SUPERSEDED status in ADR-001/ADR-003 variants; README rewritten |
| 1-F: Proof-debt classification | ✅ Done — `proofs/COVERAGE.md` has v1.0 genesis-lock proof-debt section |

Remaining genesis-lock gate: **Phase 1-G** (final evidence capture PR #239 + owner sign-off PR #240).

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

The following claims are supported by current repo artifacts (updated 2026-06-01):

- Domain A remains deterministic and `no_std` constrained.
- TX-0 and TX-1 perturbation proof obligations are represented (TV-10, TV-11 in vectors.json).
- EFB determinism and epoch-bound receipt replay rejection have Coq coverage.
- Receipt encryption uses ChaCha20-Poly1305 AEAD with domain-separated nonces; 18 tests pass.
- WAL crash-recovery is hardened: fuzz target + 5 robustness integration tests + cross-ISA CI steps.
- Hosted PAL replay, commitment transport, and whole-protocol sharded replay are implemented.
- PDF traceability is verified (Phase 1-D complete, 2026-06-01).
- Axiom classification is complete (`docs/release/v1_axiom_boundary.md`).
- Domain B backend scope is classified and bounded (`docs/release/v1_domain_b_backend_boundary.md`, ADR-013).
- Benchmark evidence suite is complete for all genesis-lock performance evidence.
- Domain B hardware/offline stubs are classified as post-v1 or demo-only behind feature gates.

## Claims Not Yet Allowed

The following claims require future evidence or owner sign-off:

- Genesis lock or deployment authority (requires PR #240 owner sign-off).
- Production networking or hardware-backed attestation (TPM/TDX/CCA/SEV-SNP are post-v1).
- Production threshold signing (TALUS is demo-only, combine_shares uses XOR placeholder).
- Production Plonky3 ZK verification (interface-only in v1.0).
- PQC migration activation (SLH-DSA epoch 10000 is defined but not activated in v1.0).
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

## Next Execution Track

PR #240 — owner-gated genesis decision.

Allowed outcomes:

A. genesis-candidate
B. RC-only milestone
C. defer

Do not update `GENESIS_CONSTANTS.toml`, flip `deployment_authoritative`, or tag
`v1.0-reference` unless the PR body contains `[genesis-change-acknowledged]`
and the owner explicitly chooses genesis-candidate.

If the owner chooses RC-only or defer, keep:

```toml
genesis_status = "provisional"
deployment_authoritative = false
```
