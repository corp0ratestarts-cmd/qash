# Work Package Breakdown — QASH Phase 1 Research Programme

**Programme framing:** Cyber-resilience substrate for deterministic replay and selective disclosure of offline incident-log commitments without publishing a transaction graph.

**Duration:** 12 months  
**Basis:** Innovate UK Industrial Research / EU Horizon EIC format

---

## WP1 — Security Review and Formal Threat Modelling (Months 1–3)

**Objective:** Independently validate the WAL format, commitment scheme, and disclosure mechanism against an adversarial threat model.

**Tasks:**
- WP1.1: External security review of `TxMvpReceiptCommit` encoding and WAL format
- WP1.2: Threat model update for import-side sync and selective disclosure
- WP1.3: Fuzz testing campaign for WAL parser (truncation, corruption, replay attacks)
- WP1.4: Constant-time audit of commitment comparison operations

**Deliverables:**
- D1.1: Security review report with findings and mitigations
- D1.2: Updated threat model (`docs/threat_model/`)
- D1.3: Extended fuzz corpus and coverage report

**Success criteria:** No P0/P1 findings unmitigated; WAL parser passes 10M fuzz iterations without crash.

---

## WP2 — Operational Integration and Relevant-Environment Validation (Months 2–6)

**Objective:** Demonstrate the system in a representative cyber-operational environment with real incident data formats.

**Tasks:**
- WP2.1: Connector for structured log sources (syslog, CEF, JSON alert formats)
- WP2.2: End-to-end pilot with a partner organisation's incident log (anonymised)
- WP2.3: Multi-node sync over realistic transports (USB, shared storage, local LAN)
- WP2.4: Performance characterisation (throughput, WAL growth, replay latency at 10k/100k records)

**Deliverables:**
- D2.1: Log source connector with format normalisation
- D2.2: Pilot report including privacy-boundary verification on partner data
- D2.3: Performance benchmark report
- D2.4: Updated `scripts/run_mvp_demo.sh` with multi-node scenario

**Success criteria:** System handles 100k records; replay completes in under 30 s; pilot partner confirms privacy properties met.

---

## WP3 — Hardening and Protocol Extension (Months 4–9)

**Objective:** Extend the system toward production robustness without crossing into Domain A consensus.

**Tasks:**
- WP3.1: WAL compaction and archival (bounded disk growth)
- WP3.2: Receipt revocation stub (mark receipt as withdrawn without disclosing body)
- WP3.3: Structured replay report with JSON output and integrity checks
- WP3.4: Workspace migration (rename vault directory, update manifests)
- WP3.5: REST/CLI gateway for integration with SIEM and SOC tooling

**Deliverables:**
- D3.1: Compaction implementation with test coverage
- D3.2: Revocation mechanism specification and prototype
- D3.3: Gateway specification and implementation

**Success criteria:** WAL growth bounded under configurable rotation policy; revocation does not reveal body; gateway passes security review.

---

## WP4 — Formal Verification Extension (Months 6–12)

**Objective:** Extend the Coq proof library to cover the MVP commitment scheme properties.

**Tasks:**
- WP4.1: Formalise `payload_commitment` binding property in Coq
- WP4.2: Formalise `nonce_commitment` collision resistance assumption
- WP4.3: Formalise replay root monotonicity (append-only WAL → strictly growing root sequence)
- WP4.4: Formalise selective disclosure soundness (disclosed receipt does not leak others)

**Deliverables:**
- D4.1–D4.4: Coq proof files under `proofs/mvp/`
- D4.5: Proof-to-implementation traceability matrix

**Success criteria:** All four proof targets compile under Coq 8.18+; traceability matrix links each proof to the corresponding Rust implementation.

---

## WP5 — Dissemination and Exploitation (Months 9–12)

**Objective:** Publish findings, engage potential adopters, and prepare Phase 2 application.

**Tasks:**
- WP5.1: Technical whitepaper (targeting IEEE S&P or USENIX Security workshop)
- WP5.2: Open-source release preparation (licence audit, documentation completeness, contribution guide)
- WP5.3: Partner engagement workshop (cyber-security operations, incident response, critical infrastructure)
- WP5.4: Phase 2 application preparation (TRL 5→6 roadmap)

**Deliverables:**
- D5.1: Whitepaper draft and submission record
- D5.2: Public repository with open-source licence
- D5.3: Workshop report and partner letters of interest
- D5.4: Phase 2 application outline

---

## Budget Indicative Summary

| WP | Activity | Indicative cost (£) |
|----|----------|---------------------|
| WP1 | Security review and formal threat modelling | 45,000 |
| WP2 | Operational integration and validation | 80,000 |
| WP3 | Hardening and protocol extension | 90,000 |
| WP4 | Formal verification extension | 60,000 |
| WP5 | Dissemination and exploitation | 25,000 |
| **Total** | | **300,000** |

*Figures are indicative for Innovate UK Industrial Research grant sizing. Actual figures depend on team composition, overhead rates, and subcontractor costs.*

---

## Claim Boundary

This work package plan does not represent a commitment to deliver a payment instrument, settlement rail, regulated product, or production critical-infrastructure deployment. All work remains within the Domain B demonstrator scope defined in `docs/mvp/claims_register.md`.
