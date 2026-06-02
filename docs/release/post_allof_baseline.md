# Post-All-Of Baseline

**Date:** 2026-06-01
**Status:** Wave 0 baseline — post-all-of-migration and post-dependency-unification state.

This document records the repository state at the start of the v1.0 genesis-candidate
completion work. It supersedes the 2026-05-30 post-GRC evidence snapshot for the purposes
of tracking what changed between that snapshot and the Wave 0 freeze point.

---

## Current Repository Head

| Field | Value |
|-------|-------|
| HEAD commit | `8b354ceae59b0bbf06cc69b9a7044b2abd615ea8` |
| HEAD PR | #217 — deps: unify RustCrypto digest stack to 0.11 and getrandom to 0.3 |
| Previous | `3efc0b7` — PR #225 — fix(pal): canonical decode, claim-boundary wording, DualHashError variants |
| Branch | `main` |
| `genesis_status` | `provisional` |
| `deployment_authoritative` | `false` |

---

## PRs merged since the 2026-05-30 post-GRC snapshot

| PR | Title | Scope |
|----|-------|-------|
| #218 | crypto(pal): add QASH hedged dual-hash utility | Domain B — PAL crypto |
| #224 | feat(pal): all-of dual-root migration — AllOfHashPair32 API + Tasks A–D | Domain B — PAL evidence |
| #225 | fix(pal): canonical decode, claim-boundary wording, DualHashError variants | Domain B — PAL corrections |
| #217 | deps: unify RustCrypto digest stack to 0.11 and getrandom to 0.3 | Dependency version bump |

---

## Note on PR #217

The v1.0 completion plan (written before Wave 0 began) assumed PR #217 was open and
non-mergeable and called for closing or deferring it before any genesis work. In practice
the PR merged into main after PR #225, and was therefore already part of HEAD when Wave 0
began.

**Risk assessment (recorded for audit trail):**

- Changed: `sha3` 0.10→0.11, `sm3` 0.4→0.5, `streebog` 0.10→0.11, `getrandom` 0.2→0.3.
- The RustCrypto `digest` 0.11 API is a compatible evolution of 0.10: same
  `new/update/finalize/reset` surface; no behavioral change for correct callers.
- `getrandom::getrandom()` renamed to `getrandom::fill()` — three call sites updated
  (`pal/mvp_vault.rs`, `pal/crypto/drbg.rs`, `src/demo.rs`); all are Domain B.
- The commit message confirms 650+ workspace tests pass and CAVP SHA3-256 + SM3
  KATs verified clean.
- Domain A (`qash-consensus`) uses these primitives indirectly through PAL; the
  no-code-change claim is consistent with a trait-compatible API bump.

**Verdict:** No evidence of behavioral divergence. The strategic concern was warranted
(consensus-adjacent deps should not be bumped casually pre-genesis), and it is recorded
here so future auditors can see the rationale and the KAT evidence. The full cascade KAT
suite, cross-ISA determinism gate, and adversarial simulation suite all continued to pass
after the merge.

---

## CI Status at PR #225 Head (3efc0b7)

All required workflows passed. Key jobs:

| Job | Outcome |
|-----|---------|
| QASH CI — full build, tests, clippy | ✅ success |
| Cross-Platform Determinism (aarch64, riscv64gc) | ✅ success |
| Pre-Genesis Full-Repo Audit — all blocking jobs | ✅ success |
| Security Compliance Preflight | ✅ success |
| Fuzz Smoke Test | ✅ success |
| MVP Demo | ✅ success |
| CAVP KAT | ✅ success |
| Domain A tripwires | ✅ success |
| Proofs (Coq) | ✅ success |
| Vector integrity | ✅ success |

Conditional/advisory jobs: skipped or success as appropriate.

---

## Open PRs at Wave 0

None. All prior stale PRs had already been merged or were never opened. No PRs
required closing or deferral in this wave.

---

## Phase 1 Prerequisites (carried forward from post-GRC snapshot)

| Item | Status |
|------|--------|
| 1-A: Genesis schema v1.0/v1.1 reconciliation | ✅ Done |
| 1-B: Stale docs reference fix | ✅ Done |
| 1-C: ADR-003 full byte-layout spec | ✅ Done |
| 1-D: Manual PDF traceability verification | ❌ Scheduled — Wave 1 (PR #227) |
| 1-E: Duplicate ADR consolidation | ✅ Done |
| 1-F: Proof-debt classification | ✅ Done |
| 1-G: Lock commit + v1.0-reference tag | ⏳ Awaiting 1-D and all upstream waves |

---

## Items deferred to post-genesis

The following items are deferred explicitly to post-genesis dependency review:

| Item | Rationale |
|------|-----------|
| Further RustCrypto digest stack changes | Any additional crypto dep bumps require full cascade KAT, genesis-hash, and cross-ISA validation before merging pre-genesis |
| sha2 0.9 / hmac-drbg 0.3 unification | No upstream hmac-drbg release for digest 0.11 exists; blocked until upstream releases |
| ML-KEM-768 hybrid for receipt encryption | Scheduled for Wave 3 (PR #231); ChaCha20-Poly1305 AEAD replaces XOR stub |
| TPM/TEE/HSM production backends | Scheduled for post-v1 classification in Wave 3 (PR #232) |
| Plonky3 full verifier implementation | Post-v1 — interface-only for v1.0 |

---

## What comes next

Wave 1 (PR #227): Reconcile `docs/traceability.md` against the committed PDF at
`spec/pdf/QASH_Spec_v1.0.pdf`, remove all provisional citation notices, and record
the PDF SHA-256. The `spec/genesis-artifacts.txt` pre-lock caveat is retained until
PR #240 (genesis-candidate decision gate).
