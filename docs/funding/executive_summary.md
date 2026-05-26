# QASH MVP — Executive Summary

**One-page summary for funding applications (Innovate UK, EU Horizon, SBRI, DSTL/NCSC)**

---

## Problem

Cyber-incident records are routinely lost, altered, or selectively disclosed under legal and regulatory pressure. Existing audit-log systems either expose a full transaction graph (revealing sensitive operational patterns) or require a trusted central authority to attest log integrity. Neither property survives an adversarial post-incident review.

---

## Solution

QASH demonstrates an **offline-first cryptographic commitment substrate** for incident-log attestation. Operators commit incident records as fixed-size cryptographic commitments to a local append-only write-ahead log. The system then exports a commitment-only public transcript — a sequence of hashes — that can be replayed deterministically across independent machines to produce a stable, tamper-evident root, without revealing the incident bodies.

Core properties:

- **No transaction-graph exposure.** The public transcript contains commitments only; the incident body and timing cannot be recovered from it.
- **Offline-first.** The full flow (init → issue-receipt → sync → replay → disclose) requires no network connectivity and operates on air-gapped or intermittently-connected hosts.
- **Deterministic replay.** The commitment root is identical across x86-64, AArch64, and RISC-V given the same input sequence — enabling cross-platform independent verification without a trusted intermediary.
- **Selective disclosure.** Individual incident bodies can be disclosed to authorised parties by receipt ID without revealing other records.

---

## Current Status (TRL 3–4)

A working local demonstrator is fully implemented and CI-verified:

| Capability | Status |
|------------|--------|
| Local incident receipt commitment | ✅ |
| Commitment-only public export | ✅ |
| Deterministic replay (cross-platform) | ✅ |
| Selective disclosure | ✅ |
| Import-side peer replay | ✅ |
| WAL corruption detection | ✅ |

Evidence: `bash scripts/run_mvp_demo.sh` passes all privacy-boundary assertions in CI on every commit.

---

## Funding Ask

**Phase 1 (12 months):** Advance from TRL 3–4 to TRL 5–6 through independent security review, operational-environment integration with a partner organisation's incident log, and performance characterisation.

Indicative budget: see `docs/funding/work_packages.md`.

---

## Claim Boundary

This system is a local Domain B demonstrator. It is not a payment instrument, settlement rail, regulated financial product, or production deployment. All allowed and blocked claims are governed by `docs/mvp/claims_register.md`.
