# Outreach Email Templates — QASH Phase 1

Use these templates for initial partner outreach. Adapt subject lines and organisation-specific details before sending. All templates use only allowed claims from `docs/mvp/claims_register.md`.

---

## Template A — SOC / MSSP Operator

**Subject:** Offline incident-log attestation research — partner opportunity

---

Dear [Name],

I am writing to explore whether [Organisation] might be interested in participating as a research partner in a cyber-resilience project we are developing.

We are building a local attestation substrate that allows security operations teams to commit incident log entries as cryptographic commitments, share a commitment-only audit trail with third parties, and selectively disclose individual incident details to authorised parties — without publishing a transaction graph or exposing unrelated incidents.

The system is designed to work offline and in intermittently-connected environments, making it practical for air-gapped SOC infrastructure and data-diode deployments.

We are looking for one or two SOC operators who would be willing to participate in a pilot during the first half of 2027: providing a representative sample of anonymised incident log data, running the demonstrator against it, and confirming whether the privacy-boundary properties hold in a realistic operational context.

Participation would be covered by a standard bilateral NDA and a brief MOU covering data use and publication rights. There is no commercial commitment on either side.

If this sounds like it might be a fit, I would welcome a 30-minute call to learn more about your environment and share a technical overview.

Kind regards,  
[Name]  
[Organisation]  
[Contact details]

---

## Template B — OT / Critical Infrastructure Operator

**Subject:** Air-gapped audit trail integrity — research collaboration

---

Dear [Name],

I am reaching out about a research project focused on cyber-resilience infrastructure for operational technology environments.

We are developing an offline-first attestation substrate that produces a tamper-evident audit trail for incident log entries, verifiable without network connectivity. The approach is designed for environments where continuous connectivity cannot be assumed — including air-gapped systems, data-diode architectures, and intermittently-connected field devices.

The substrate produces a commitment-only public transcript: third parties (regulators, auditors, insurers) can verify the integrity and deterministic replay of the audit trail without seeing private operational details. Individual incident records can be selectively disclosed at appropriate milestones.

We are exploring whether operators in your sector would be willing to participate in a technical pilot: sharing a representative anonymised log sample and providing feedback on whether the system addresses real operational needs.

Would you be open to a brief conversation? I am happy to share a technical overview and discuss what participation might involve.

Best regards,  
[Name]  
[Organisation]  
[Contact details]

---

## Template C — Incident Response Firm

**Subject:** Evidence integrity and selective disclosure — research partnership

---

Dear [Name],

I am writing about a research project that may be relevant to your incident response practice.

We are building a commitment-based evidence integrity system for incident logs. During an engagement, individual incident records are committed locally as cryptographic commitments. A commitment-only public transcript can be shared with counsel, regulators, or insurers to establish chain-of-custody without premature disclosure of case details. Individual records can then be selectively disclosed at agreed milestones, with each disclosure independently verifiable against the original commitment.

The system does not require network connectivity and produces a deterministic replay root that can be independently verified — potentially useful for cross-party evidence verification in multi-jurisdictional cases.

We are looking for a research partner with IR experience who could advise on whether the approach meets real chain-of-custody requirements and participate in a brief technical pilot.

Would you be interested in a 30-minute call to explore this?

With thanks,  
[Name]  
[Organisation]  
[Contact details]

---

## Letter of Support Template

**[Organisation letterhead]**

**Date:** [Date]

**Re:** Letter of Support — QASH Phase 1 Research Programme

This letter confirms that [Organisation] has reviewed the proposed research programme described in the attached technical annex and is supportive of [Lead Organisation]'s application for [funding programme].

[Organisation] operates [brief description of OT/SOC/IR context]. We have identified the proposed cyber-resilience substrate — specifically the offline incident-log attestation capability and commitment-based selective disclosure mechanism — as potentially relevant to our operational audit trail and evidence integrity requirements.

Subject to mutual agreement on terms, [Organisation] is willing to:

- Provide a representative anonymised sample of incident log data for use in the WP2 pilot
- Participate in a technical review of the demonstrator output
- Attend the WP5 partner engagement workshop
- Provide a technical assessment of the system's fit for our operational context

This letter does not constitute a commercial or financial commitment. Our participation would be governed by a separate non-disclosure agreement and memorandum of understanding.

Signed: [Authorised signatory]  
Title: [Title]  
Organisation: [Organisation]  
Date: [Date]
