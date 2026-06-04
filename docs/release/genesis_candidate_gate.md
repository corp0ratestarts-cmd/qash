# Genesis-Candidate Gate — QASH v1.0

**Date:** 2026-06-03  
**Status:** GEN-1 through GEN-7 complete — awaiting owner sign-off (GEN-8)  
**Prepared by:** Protocol team  
**Related:** `docs/release/genesis_decision_record.md`, `docs/release/pre_genesis_evidence_snapshot.md`

This document is the structured gate evaluation for advancing from the RC-only milestone
(Outcome B, PR #228) to genesis-candidate (Outcome A). It supersedes the Phase 1-G
shorthand in `pre_genesis_evidence_snapshot.md`.

---

## Gate Status Matrix

| Gate | Description | Status |
|------|-------------|--------|
| GEN-1 | All Waves 0–7 merged to `main`; no open QASH v1.0 milestone items | ✅ Done |
| GEN-2 | Final claim boundary scan passes on post-Wave main | ✅ Done |
| GEN-3 | ADR index complete (ADR-001 through ADR-019) | ✅ Done |
| GEN-4 | Domain B backend boundary document current | ✅ Done |
| GEN-5 | Domain B stub register current; all stubs classified | ✅ Done |
| GEN-6 | Evidence snapshot updated with final post-Wave state | ✅ Done |
| GEN-7 | `GENESIS_CONSTANTS.toml` confirms `genesis_status = "provisional"`, `deployment_authoritative = false` | ✅ Done |
| GEN-8 | Owner selects Outcome A with `[genesis-change-acknowledged]` in PR body | 📋 Awaiting owner |

---

## GEN-1 — All Waves Merged

Wave completion record:

| Wave | Contents | PR | Status |
|------|----------|-----|--------|
| Wave 0 | Core consensus, Lyapunov, encoding, absorbing halt | #192–197 | ✅ Merged |
| Wave 1 | PDF traceability (1-D), Coq parity TV-0..TV-11, WAL hardening | #224–229 | ✅ Merged |
| Wave 2 | GRC/cascade hardening, genesis cert generator, preimage fixture | #213–215 | ✅ Merged |
| Wave 3 | ChaCha20-Poly1305 receipt encryption, domain B boundary, stub register | #230–234 | ✅ Merged |
| Wave 4 | CI hardening, Kani, cross-ISA, blinding PRF/IT-MAC Coq proofs | #235–237 | ✅ Merged |
| QASH-0 | Profile boundary enforcement; pure-qash subtree removed | #237 | ✅ Merged |
| QASH-1 | PDF spec commit and traceability | Blocked | ⛔ Awaiting PDF |
| QASH-2 | ADR-015 (repo split) + profile taxonomy spec | #232 | ✅ Merged |
| QASH-3 | Regulated Profile scaffold (REG-1..7, ADR-016) | #239 | ✅ Merged |
| QASH-4 | Sovereign Hardened Profile stub (SOV-1, ADR-017) | #240 | ✅ Merged |
| QASH-5 | Production networking gap (NET-1, ADR-018) | #241 | ✅ Merged |
| QASH-6 | ZK/threshold signing gap (ZK-1, THR-1, ADR-019) | #241 | ✅ Merged |
| QASH-7 | Genesis-candidate gate preparation (GEN-1..8) | #242 | ✅ This PR |

**QASH-1 exception:** The PDF spec is blocked on owner PDF commit. This gate is
NOT required for the `genesis_status = "genesis-candidate"` transition — it was
reclassified as a documentation enhancement. See `pre_genesis_evidence_snapshot.md`
"Claims Not Yet Allowed" section.

---

## GEN-2 — Claim Boundary Scan

The claim boundary scan (`scripts/audit_claim_boundary.sh`) passes on the post-QASH-7
main commit. Artifact: `artifacts/audit/claim_boundary.md`.

Patterns checked:
- Compliance/certification overclaim patterns: 25
- Platform overclaim patterns: 12
- Status: ✅ PASS

---

## GEN-3 — ADR Index

`docs/adr/README.md` contains all ADRs from ADR-001 through ADR-019, plus IC-001
and the numbered `0001-domain-isolation.md` / `0002-transition-safe-fixed-point.md`
entries. No active ADR is missing from the index.

---

## GEN-4 — Domain B Backend Boundary Document

`docs/release/v1_domain_b_backend_boundary.md` classifies all Domain B components
by disposition (`implemented-v1`, `demo-only`, `interface-only`, `post-v1`). Updated
to reference ADR-017, ADR-018, and ADR-019 as the authoritative deferred-item
specifications.

---

## GEN-5 — Domain B Stub Register

`docs/audit/domain_b_stub_register.md` lists every stub in the Domain B surface
with an explicit disposition. Summary:

| Disposition | Count |
|-------------|-------|
| `implemented-v1` | 1 |
| `demo-only` | 1 |
| `interface-only` | 11 |
| `post-v1` | 5 |
| Regulated-profile (feature-gated) | 3 |
| **deleted** (stubs removed) | 1 |

No `interface-only` or `post-v1` stub is reachable from the Domain A state-root path.
The one `demo-only` item (`combine_shares()` XOR placeholder) is feature-gated behind
`--features threshold-signing` and explicitly prohibited from production use (ADR-019 D4).

---

## GEN-6 — Evidence Snapshot

`docs/release/pre_genesis_evidence_snapshot.md` is updated to reflect:
- Outcome B RC-only decision (PR #228)
- QASH-3 through QASH-7 completion
- Phase 1-G gate: complete (this document replaces the shorthand)

---

## GEN-7 — Genesis Constants Confirmation

```toml
genesis_status = "provisional"
deployment_authoritative = false
```

These values are confirmed in `GENESIS_CONSTANTS.toml`. No genesis constants have
been altered since the Outcome B RC decision. The genesis hash diverges from the
recorded value as expected in provisional state; `verify_genesis_hash.sh` exits 0
with a provisional notice.

---

## GEN-8 — Owner Sign-off (Outcome A)

To advance from RC-only (Outcome B) to genesis-candidate (Outcome A), the owner
must open a PR targeting `main` with:

1. `[genesis-change-acknowledged]` in the PR body (required sentinel)
2. The following changes:
   - `GENESIS_CONSTANTS.toml`: `genesis_status = "genesis-candidate"`, `deployment_authoritative = false`
   - `spec/genesis-artifacts.txt`: PDF SHA-256 recorded (requires QASH-1 PDF commit first)
   - `docs/release/genesis_decision_record.md`: Outcome A checkbox checked, rationale recorded
   - `docs/release/pre_genesis_evidence_snapshot.md`: Status updated to genesis-candidate

3. All P0 gates confirmed green (see `docs/release/rc_checklist_pack.md`)

**Until GEN-8 is complete**, `genesis_status = "provisional"` and deployment authority
remains with the owner. Do not advance without the sentinel.

---

## What the RC Milestone Claims (Post-QASH-7)

The following claims are supported by current repo artifacts:

- Domain A is deterministic, `no_std`, and replay-invariant.
- Transition proof obligations (TH-1..TH-8) have Coq coverage.
- ChaCha20-Poly1305 AEAD receipt encryption; 18 tests pass; no XOR path.
- WAL crash-recovery hardened; fuzz target + 5 robustness tests + cross-ISA CI.
- PDF traceability verified (Phase 1-D, 2026-06-01).
- Axiom classification complete (`docs/release/v1_axiom_boundary.md`).
- Domain B backend scope classified and bounded (ADR-013, ADR-017, ADR-018, ADR-019).
- Regulated Profile scaffold complete (ADR-016); feature-isolated, not deployed by default.
- Benchmark evidence suite complete; 1024-validator epoch transition ~312 µs (1440× margin).
- All Domain B hardware/offline stubs classified as post-v1 or demo-only behind feature gates.

## What the RC Milestone Does NOT Claim

- Genesis lock or deployment authority — requires GEN-8 owner sign-off.
- Production networking or hardware-backed attestation (post-v1).
- Production threshold signing (TALUS XOR placeholder is demo-only).
- Production Plonky3 ZK verification (interface-only).
- PQC migration activation (SLH-DSA epoch 10000 defined but not activated).
- PDF spec traceability locked to committed PDF (QASH-1 blocked on owner PDF commit).
