# Budget Detail Template — QASH Funding Application

**[TEMPLATE — complete bracketed fields before submission]**

---

## Summary

| Category | Amount (£) | % of Total |
|---|---|---|
| Personnel | [AMOUNT] | [%] |
| Subcontractors / Pilot Partners | [AMOUNT] | [%] |
| Infrastructure and tooling | [AMOUNT] | [%] |
| Dissemination and reporting | [AMOUNT] | [%] |
| Overheads | [AMOUNT] | [%] |
| **Total** | **[TOTAL]** | **100%** |

---

## Personnel

| Role | Days | Day Rate (£) | Total (£) | Justification |
|---|---|---|---|---|
| Principal Investigator / Lead Engineer | [DAYS] | [RATE] | [TOTAL] | Protocol design, formal assurance, pilot coordination |
| Security Engineer (post-quantum crypto) | [DAYS] | [RATE] | [TOTAL] | Dilithium5/SLH-DSA production integration |
| Systems Engineer (embedded/RISC-V) | [DAYS] | [RATE] | [TOTAL] | Cross-platform determinism, hardware attestation |

---

## Subcontractors / Pilot Partners

| Partner | Role | Amount (£) |
|---|---|---|
| [PILOT PARTNER 1] | Operator pilot, feedback, case study | [AMOUNT] |
| [PILOT PARTNER 2] | Independent replay verification | [AMOUNT] |

---

## Infrastructure and Tooling

| Item | Amount (£) | Justification |
|---|---|---|
| CI/CD infrastructure (12 months) | [AMOUNT] | Cross-platform build and test on x86-64/AArch64/RISC-V |
| Hardware attestation testbed (TPM 2.0) | [AMOUNT] | Phase 1 attestation integration |
| Formal verification compute (Coq) | [AMOUNT] | TH-9/TH-10/TH-11 proof completion |

---

## Dissemination and Reporting

| Item | Amount (£) |
|---|---|
| Final report and evidence bundle | [AMOUNT] |
| Conference / workshop attendance | [AMOUNT] |
| Public case study publication | [AMOUNT] |

---

## Value for Money Justification

- The core protocol is already implemented and CI-verified (TRL 3–4), so funding covers integration and assurance, not speculative research.
- Cross-platform determinism is verified automatically on every commit — no manual testing overhead.
- The open-source codebase (GPL-3.0) ensures public benefit beyond the grant period.
- Pilot partners contribute in-kind evaluation effort, reducing cash cost.

---

All cost claims are consistent with the scope defined in `docs/funding/application_narrative.md`.  
All technical claims are governed by `docs/mvp/claims_register.md`.
