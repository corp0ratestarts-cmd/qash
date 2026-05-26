# Defence and National-Security Assurance Roadmap

**Status:** Scoping document — not a certification claim.  
**Scope:** Defence, national-security, military-adjacent, classified-environment, and high-assurance procurement positioning for QASH.  
**Audience:** Grant writers, assessors, defence/commercialisation partners, auditors, and future certification advisors.

---

## 0. Non-Claims Boundary

This document records potential certification, accreditation, and assurance pathways that QASH may be designed to support. It does **not** claim that QASH, QASH Labs, or any QASH deployment is currently certified, accredited, approved, authorised, endorsed, or validated by NSA, NCSC, NATO, DoD, DISA, FedRAMP, NIAP, CMVP, ISO, SOC 2, or any other authority.

The permitted claim is:

> QASH is being designed as a replay-deterministic, formally evidenced, compliance-mapped infrastructure substrate whose technical evidence could support future certification and defence-assurance pathways.

The prohibited claims are:

- "NSA approved"
- "military certified"
- "classified-system ready"
- "FedRAMP authorised"
- "FIPS validated"
- "Common Criteria certified"
- "CMMC compliant"
- "NATO certified"
- "GDPR compliant"
- any equivalent statement unless a completed third-party certification, accreditation, validation, or authority-to-operate exists.

---

## 1. Strategic Rationale

QASH is not only a protocol R&D project. Its long-term commercial value depends on converting technical assurance into repeatable evidence for procurement, partner validation, regulated-sector adoption, and high-assurance certification work.

QASH's design choices are relevant to defence and national-security assurance because they emphasise:

- deterministic replay across authorised instruction-set architectures;
- formal proof artefacts and proof-to-code traceability;
- public transcript minimisation;
- zero-persistence and erasure-compatible receipt handling;
- supply-chain evidence, SBOMs, pinned toolchains, reproducible builds, and release attestations;
- cryptographic-module boundary separation between Domain A and Domain B;
- post-quantum cryptographic readiness;
- audit-ready evidence bundles per release candidate.

These properties are not a substitute for certification. They are inputs into a certification-readiness and procurement-readiness programme.

---

## 2. NSA CNSA 2.0 Alignment

### Positioning

QASH should position toward **CNSA 2.0-aligned cryptographic posture**, not NSA approval.

### Current QASH relevance

- Domain B already contains an ML-KEM-768 roadmap and implementation direction.
- FIPS-aligned crypto documentation maps ML-KEM-768, SHA3-256, HMAC-DRBG, and future signature work.
- The protocol architecture separates deterministic Domain A consensus from operational Domain B cryptographic services.

### Gap to close

- Add an explicit `docs/compliance/cnsa_2_alignment.md` mapping.
- Complete ML-KEM-768 KAT evidence.
- Add ML-DSA / SLH-DSA roadmap where signatures are required.
- Keep CNSA claims as alignment only until products/components are evaluated through the appropriate channels.

### Permitted claim

> QASH is being designed toward CNSA 2.0-aligned post-quantum cryptographic posture in Domain B.

---

## 3. NSA CSfC-Adjacent Architecture Path

### Positioning

Commercial Solutions for Classified (CSfC) is an architecture/component approval pathway, not a generic property of good cryptography. QASH should therefore position as **CSfC-adjacent** or **CSfC-architecture-mappable**, not CSfC-approved.

### Potential QASH relevance

- Offline evidence integrity and commitment verification.
- Secure transport profiles for commitment synchronisation.
- Post-quantum key-establishment direction.
- Release attestation and reproducible build evidence.
- Separation of deterministic core and operational boundary.

### Gap to close

- Identify which QASH components could become evaluated commercial components.
- Map any deployment architecture to relevant NSA Capability Packages.
- Engage CSfC Trusted Integrator expertise if classified or mission-network use becomes a real market path.
- Ensure underlying crypto modules/components have appropriate NIAP/FIPS/CMVP evidence where required.

### Permitted claim

> QASH may be suitable for future CSfC-adjacent architecture mapping if packaged as evaluated components and integrated through the appropriate CSfC process.

---

## 4. NIAP / Common Criteria / ISO 15408 Path

### Positioning

Common Criteria is one of the strongest routes for procurement-grade security assurance if the Target of Evaluation (TOE) is narrow and well-bounded.

### Recommended TOE candidates

1. **QASH Offline Evidence Verifier** — deterministic replay verifier for public commitment transcripts.
2. **QASH Domain B Evidence Module** — local incident-receipt commitment, selective disclosure, and erasure evidence module.
3. **QASH Cryptographic PAL Module** — Domain B crypto boundary, KATs, self-tests, and key handling.
4. **QASH Evidence Appliance Profile** — hardened offline appliance deployment profile.

### Gap to close

- Write `docs/compliance/cc_security_target.md`.
- Define TOE boundary and security problem definition.
- Map Security Functional Requirements (SFRs), e.g. FCS_CKM, FCS_COP, FPT_TST, FAU_GEN, FDP_ACC/FDP_ACF where appropriate.
- Map Security Assurance Requirements (SARs), e.g. ADV_ARC, ADV_FSP, ATE_COV, ATE_FUN, AVA_VAN.
- Select an evaluation lab and target Evaluation Assurance Level only after the TOE is stable.

### Permitted claim

> QASH is being prepared for a future Common Criteria / ISO 15408 security-target strategy around a narrowly bounded evidence-verification or Domain B module.

---

## 5. FIPS 140-3 / CMVP and CAVP / ACVP Path

### Positioning

FIPS validation applies to a cryptographic module boundary and requires lab validation. QASH should claim FIPS alignment and test-evidence preparation only until CMVP validation exists.

### Current QASH relevance

- `docs/compliance/fips_compliance.md` maps a Domain B cryptographic boundary.
- Domain B includes HMAC-DRBG, ML-KEM-768 direction, KATs, TLS policy work, and pseudonymised logging direction.
- CAVP/ACVP-style KAT evidence is planned in CI.

### Gap to close

- Finalise the cryptographic module boundary.
- Generate algorithm validation evidence for approved algorithms.
- Add power-on self-tests and conditional self-tests.
- Add constant-time testing, e.g. dudect-style evidence.
- Engage a CSTL/CMVP lab only after the module boundary and algorithms are stable.

### Permitted claim

> QASH Domain B is being developed toward FIPS 140-3-aligned cryptographic-module evidence and future CAVP/CMVP readiness.

---

## 6. CMMC / NIST SP 800-171 Readiness

### Positioning

CMMC and NIST SP 800-171 are organisational and operational readiness pathways for handling Federal Contract Information (FCI) or Controlled Unclassified Information (CUI). They are not purely product certifications.

### Potential QASH Labs relevance

- Necessary if QASH Labs pursues US defence-industrial-base partners, primes, or programmes involving CUI/FCI.
- QASH technical outputs can support auditability, incident evidence preservation, access accountability, integrity controls, and supply-chain evidence.

### Gap to close

- Define company system boundary for FCI/CUI handling.
- Establish policies for access control, incident response, audit logging, media protection, configuration management, personnel security, and risk assessment.
- Maintain SSP, POA&M, asset inventory, and evidence repository.
- Seek CMMC Level 1 or Level 2 readiness only when a defence-industrial opportunity requires it.

### Permitted claim

> QASH Labs can pursue CMMC / NIST SP 800-171 readiness if defence-industrial work involving FCI or CUI becomes a target market.

---

## 7. NIST SP 800-53 / FedRAMP / OSCAL Evidence Path

### Positioning

FedRAMP is relevant only if QASH becomes a cloud service or hosted verification/evidence-management platform. For the current offline/local MVP, the relevant near-term work is NIST SP 800-53-style control mapping and OSCAL-style evidence capture.

### Potential QASH relevance

- Hosted verifier or evidence portal for government users.
- Release evidence bundles mapped to access control, audit, configuration, incident response, system integrity, and supply-chain controls.
- OSCAL-style machine-readable evidence output.

### Gap to close

- Decide whether QASH Labs will offer a hosted service.
- If yes, define cloud boundary, shared-responsibility model, SSP, continuous monitoring, and control baseline.
- Add `docs/compliance/nist_800_53_mapping.md` and OSCAL export support.

### Permitted claim

> QASH is being designed so its evidence bundles could support future NIST SP 800-53 / FedRAMP-style control evidence if a hosted government-facing service is pursued.

---

## 8. DISA STIG / SRG Deployment Hardening

### Positioning

STIG/SRG alignment is deployment hardening, not a protocol property. It becomes relevant if QASH is packaged for DoD, air-gapped, military-adjacent, or hardened appliance deployments.

### Potential QASH relevance

- Linux host hardening for QASH validator/verifier nodes.
- Container or appliance configuration baselines.
- TLS configuration, logging controls, account management, file permissions, dependency controls, and audit forwarding.
- Offline evidence appliance profile.

### Gap to close

- Create hardened deployment guides.
- Map Linux/container configuration to relevant STIG/SRG controls.
- Add automated configuration checks.
- Capture STIG-like compliance evidence in release bundles.

### Permitted claim

> QASH can support future DISA STIG/SRG-style deployment hardening profiles for defence-adjacent or hardened offline deployments.

---

## 9. DoD Impact Levels / IL2, IL4, IL5, IL6

### Positioning

DoD Impact Levels are relevant only for cloud-hosted deployments or services handling DoD information categories. They are not relevant to the current local/offline MVP unless a hosted government service is pursued.

### Potential QASH relevance

- IL2: public or low-sensitivity hosted verification services.
- IL4/IL5: CUI-related evidence-management services.
- IL6: classified environments — long-term only, requiring entirely different authority, hosting, and operational constraints.

### Gap to close

- Hosted service decision.
- Data classification boundary.
- Cloud provider and environment choice.
- FedRAMP/DoD authorisation strategy.

### Permitted claim

> If QASH becomes a hosted government-facing service, deployment profiles could be mapped to DoD cloud impact-level requirements; this is not part of the current MVP claim boundary.

---

## 10. NATO / Allied Defence Assurance Mapping

### Positioning

There is no single generic "NATO certification" for QASH. The correct strategy is allied-defence assurance mapping through recognised national and international frameworks.

### Potential QASH relevance

- FIPS/CAVP and CNSA-style cryptographic posture.
- Common Criteria / ISO 15408 TOE strategy.
- NCSC CAF and national cyber-resilience frameworks.
- SBOM, SLSA, OpenSSF, reproducible builds, and supply-chain assurance.
- Secure-by-design and incident-evidence preservation for NATO/allied environments.

### Gap to close

- Identify target country and procurement route.
- Map to the relevant national security authority, cyber framework, or procurement baseline.
- Avoid generic claims of NATO certification unless a specific programme/standard is identified.

### Permitted claim

> QASH can be positioned for NATO/allied defence assurance alignment through recognised national and international frameworks, not through a generic NATO certification claim.

---

## 11. UK Defence and National Cyber Pathways

### Positioning

UK-facing defence and national cyber pathways should be framed around cyber resilience, trustworthy infrastructure, secure-by-design development, evidence integrity, and public-sector assurance.

### Potential QASH relevance

- NCSC CAF mapping.
- Cyber Essentials / Cyber Essentials Plus for organisational baseline.
- Secure-by-design and software supply-chain assurance.
- DSTL / Defence and Security Accelerator / SBRI-style R&D calls where the call scope matches offline evidence integrity, cyber resilience, secure logging, or trusted infrastructure.

### Gap to close

- Company-level Cyber Essentials / Plus readiness.
- UK public-sector pilot partner or use-case.
- Procurement-ready evidence pack.
- Clear non-financial, non-token, non-payment claim boundary.

### Permitted claim

> QASH is suitable for assessment against UK cyber-resilience, defence innovation, and public-sector assurance pathways where offline evidence integrity and replay-verifiable infrastructure are in scope.

---

## 12. Evidence Bundle Requirements

A defence-assurance evidence bundle should include:

- release commit hash;
- SBOM;
- OSV/dependency vulnerability scan;
- CodeQL/SAST results;
- OpenSSF Scorecard;
- proof hash manifest;
- Coq proof status and admitted-marker scan;
- reproducible build attestation;
- cross-ISA replay roots;
- KAT/CAVP outputs;
- constant-time audit output where applicable;
- fuzz and Kani summaries;
- zero-persistence boundary summary;
- privacy-boundary summary;
- configuration-hardening summary;
- OSCAL/control-mapping output where available;
- claims-boundary statement.

---

## 13. Recommended Roadmap Additions

| Document | Purpose |
|---|---|
| `docs/compliance/cnsa_2_alignment.md` | PQC and CNSA 2.0 alignment map |
| `docs/compliance/cc_security_target.md` | Common Criteria TOE and Security Target draft |
| `docs/compliance/nist_800_53_mapping.md` | FedRAMP / NIST 800-53 / OSCAL control mapping |
| `docs/compliance/cmmc_readiness.md` | Company-level CMMC/NIST SP 800-171 readiness plan |
| `docs/compliance/stig_srg_profile.md` | Hardened deployment profile and STIG/SRG mapping |
| `docs/compliance/defence_evidence_bundle.md` | Defence-facing release evidence bundle format |

---

## 14. Funding and ROI Framing

The defence-assurance pathway is strategically important because it turns grant-funded R&D outputs into reusable commercial assets:

- evaluated technical evidence for enterprise and public-sector procurement;
- credible route into defence-industrial and national-security innovation programmes;
- reduced friction for later certification and audit work;
- stronger due-diligence posture for partners and funders;
- reusable evidence bundles across UK, EU, US, and allied procurement contexts.

Grant-facing positioning should therefore emphasise:

> QASH aims to convert formally verified, replay-deterministic infrastructure into an evidence-generating platform that can support future certification, defence-assurance, and regulated-sector procurement pathways.
