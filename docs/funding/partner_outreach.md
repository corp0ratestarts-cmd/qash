# Partner Outreach Strategy — QASH Phase 1

**Objective:** Identify and engage partner organisations for the WP2 pilot and WP5 workshop.

---

## Target Partner Profiles

### Profile A — Cyber Security Operations Centres (SOCs)

**Why:** SOCs produce large volumes of structured incident logs. A privacy-preserving commit-and-replay substrate addresses the need to share audit trails with insurers, regulators, and external investigators without exposing sensitive operational details.

**Value proposition:**
- Share commitment-only transcripts with third parties without disclosing incident bodies
- Verify audit trail integrity offline without network dependency
- Selective disclosure to named investigators without exposing unrelated incidents

**Target organisations:**
- MSSP (Managed Security Service Provider) operators
- In-house SOC teams at FTSE 250 / critical national infrastructure operators
- Public sector SOC operators (NHS Digital, local authority cyber teams)

---

### Profile B — Operational Technology (OT) and Critical Infrastructure

**Why:** OT environments are often air-gapped or intermittently connected. Existing audit solutions assume persistent network connectivity. QASH's offline-first design is a direct fit.

**Value proposition:**
- Full attestation flow with no network dependency
- Tamper-evident local audit log for regulatory compliance (NIS2, CAF, NERC CIP)
- Import-side sync for one-way data diode environments

**Target organisations:**
- Energy sector (grid operators, generation asset owners)
- Water utilities
- Transport operators (rail, aviation ground systems)
- Defence prime contractors

---

### Profile C — Incident Response and Digital Forensics

**Why:** IR engagements require evidence integrity guarantees. A commitment-only public transcript allows chain-of-custody verification without premature disclosure of case details.

**Value proposition:**
- Commitment-based chain-of-custody before formal disclosure
- Selective disclosure to counsel, regulators, or insurers at agreed milestones
- Deterministic replay for independent evidence verification

**Target organisations:**
- Incident response firms (Tier 1 IR retainers)
- Digital forensics laboratories
- Cyber insurance underwriters and claims assessors

---

## Outreach Process

1. **Initial contact:** Email or LinkedIn outreach using templates in `outreach_email_templates.md`
2. **Discovery call:** 30-minute call to qualify fit and data-sharing appetite
3. **NDA:** Standard bilateral NDA before sharing technical details
4. **Pilot agreement:** Brief MOU covering data use, anonymisation, and publication rights
5. **Letter of Support:** For funding applications, a one-page letter of support confirming partner interest and participation intent

---

## Key Messages (Funding-Safe)

Use these messages in partner conversations. Do not use blocked terms from the claims register.

✅ Use:
- "cyber-resilience substrate"
- "offline incident-log attestation substrate"
- "commitment-only public audit trail"
- "deterministic replay for independent verification"
- "selective disclosure without a transaction graph"
- "air-gapped and intermittently-connected environments"
- "offline-first cryptographic commitment"

❌ Avoid (claim boundary violations):
- "blockchain", "distributed ledger", "cryptocurrency"
- "payment", "settlement", "clearing", "token", "coin"
- "production-ready", "certified", "deployed", "genesis-admitted"
- "identity provider", "credential issuer"
- "production ZK", "production hardware attestation"
- "custody", "wallet"

---

## Timeline

| Month | Activity |
|-------|----------|
| 1 | Identify 10 candidate partners across three profiles |
| 2 | Initial outreach to all 10; qualify 3–5 to discovery call |
| 3 | Discovery calls; select 1–2 pilot partners; execute NDAs |
| 4–5 | Pilot agreement and data access arrangement |
| 6 | Begin WP2.2 pilot with partner data |
| 10 | WP5.3 partner engagement workshop |
| 12 | Letters of interest for Phase 2 application |

---

All partner-facing claims are governed by `docs/mvp/claims_register.md`.
