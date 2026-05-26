# Compliance Mapping — QASH

This document maps QASH capabilities to relevant regulatory and standards frameworks. It is a **scoping document**, not a compliance certification.

---

## NIS2 Directive (EU) / UK NIS Regulations

| NIS2 Requirement | QASH Relevance | Gap |
|---|---|---|
| Incident reporting with evidence trail | Public commitment transcript provides tamper-evident record | Production integration required |
| Audit log integrity | Write-ahead log with append-only invariant | Production deployment not yet validated |
| Incident evidence preservation | Offline-first design, no network dependency | Only demonstrator; no production key management |
| Independent verification of evidence | Cross-platform deterministic replay | Third-party partner validation pending |

**Assessment:** QASH addresses the audit-trail integrity and independent verification requirements architecturally. A production deployment would need to integrate with the operator's incident reporting workflow.

---

## DORA (Digital Operational Resilience Act — EU)

| DORA Article | QASH Relevance | Gap |
|---|---|---|
| Art. 17 — ICT incident management | Commitment-only public transcript for incident records | Not a complete incident management system |
| Art. 19 — Incident reporting | Selective disclosure enables reporting without full log exposure | Production workflow integration needed |
| Art. 24 — Digital operational resilience testing | Deterministic replay enables independent test verification | Pilot only; not production-certified |

**Assessment:** DORA is a potential secondary market after NIS2 traction. Do not lead with DORA framing in UK-primary applications.

---

## NCSC Cyber Assurance Framework (CAF)

| CAF Objective | QASH Relevance |
|---|---|
| B3: Asset Management — maintain accurate records | Commitment transcript provides tamper-evident asset log |
| C1: Security Monitoring — detect incidents | Out of scope (QASH is for evidence commitment, not detection) |
| C2: Proactive Security — respond to incidents | Selective disclosure supports controlled response reporting |
| D1: Response and Recovery — recover effectively | Offline-first design supports air-gapped recovery scenarios |

---

## Privacy-by-Design (UK GDPR / Data Protection Act 2018)

| Principle | QASH Implementation |
|---|---|
| Data minimisation | Public transcript contains commitment roots only; no personal data |
| Purpose limitation | Commitment transcript is for audit integrity only, not analytics |
| Storage limitation | Artifacts are non-persistent; no cloud storage by design |
| Integrity and confidentiality | Selective disclosure isolates individual records |
| Accountability | Claims register and evidence manifest provide audit trail |

**Assessment:** The architecture is aligned with privacy-by-design principles. A formal DPIA would be required before production deployment with real incident data.

---

## Software Supply Chain Controls

| Control | QASH Implementation | Standard |
|---|---|---|
| Dependency vulnerability scan | OSV scan on every PR | NIST SSDF |
| Software Bill of Materials (SBOM) | Generated on every PR | EO 14028 / UK Cyber Strategy |
| Pinned toolchain | `rust-toolchain.toml` | SLSA Level 2 |
| OpenSSF Scorecard | Run on every PR | OpenSSF Best Practices |
| CodeQL static analysis | Run on every PR | OWASP SAST |

---

All claims in this document are governed by `docs/mvp/claims_register.md`.  
This document does not constitute a compliance certification for any regulatory framework.
