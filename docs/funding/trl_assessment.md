# TRL Assessment — QASH MVP

**Assessment date:** 2026-05-25  
**Scope:** QASH offline incident receipt commit demonstrator (Domain B only)

---

## TRL Scale Reference

| TRL | Description |
|-----|-------------|
| 1 | Basic principles observed |
| 2 | Technology concept formulated |
| 3 | Experimental proof of concept |
| 4 | Technology validated in lab |
| 5 | Technology validated in relevant environment |
| 6 | Technology demonstrated in relevant environment |
| 7 | System prototype demonstrated in operational environment |
| 8 | System complete and qualified |
| 9 | Actual system proven in operational environment |

---

## Current Assessment: TRL 3–4

### TRL 3 — Experimental Proof of Concept ✅

**Achieved.** The MVP demonstrator confirms the core technical hypotheses:

- Incident bodies can be committed locally without exposing raw content in the public transcript
- A commitment-only public export can be replayed deterministically across machines and ISAs
- Selective disclosure of a single receipt is possible without revealing other receipts
- The system operates entirely offline

Evidence: `bash scripts/run_mvp_demo.sh` passes all four privacy-boundary assertions.

### TRL 4 — Technology Validated in Lab ✅ (partial)

**Partially achieved.** The implementation is validated by an automated test suite:

- Unit tests for all cryptographic primitives (`TX-MVP-ReceiptCommit` encode/decode, commitments, public export)
- Vault integration tests (issue, sync, replay, disclose)
- Corruption tests (truncated WAL, wrong magic, duplicate records, unknown receipt ID)
- Import-side sync test (peer node replicates replay root without holding private bodies)
- Cross-platform build verification (x86-64, AArch64, RISC-V via CI)

**Not yet achieved at TRL 4:**
- No independent third-party code review
- No formal threat model review for the WAL format
- No performance characterisation under load

---

## Gap Analysis to TRL 5

To reach TRL 5 (technology validated in relevant environment), the following would be required:

| Item | Effort estimate |
|------|----------------|
| Independent security review of WAL format and commitment scheme | 2–4 weeks (external reviewer) |
| Integration with a representative operational log source (e.g., syslog, SIEM export) | 4–6 weeks |
| Performance characterisation: throughput, WAL size growth, replay latency | 1–2 weeks |
| Multi-node sync over a realistic transport (file share, USB, local network) | 3–4 weeks |
| Formal threat model update for the import path and disclosure mechanism | 1–2 weeks |

---

## What This TRL Claim Supports

This TRL 3–4 assessment supports funding applications at the **proof-of-concept / feasibility** stage:

- Innovate UK Smart Grants (Feasibility / Industrial Research phases)
- EU Horizon Europe EIC Accelerator (feasibility component)
- SBRI Phase 1 (demonstrator contracts)
- DSTL / NCSC early-stage research grants

It does **not** support claims of operational readiness, product launch, or deployment in critical infrastructure.

---

## Claim Boundary

This TRL assessment applies only to the `TX-MVP-ReceiptCommit` demonstrator (Domain B). The broader QASH consensus protocol (Domain A) has not been assessed for TRL; it remains in active research and formal verification.
