# Funding Application Narrative — QASH

**For:** Innovate UK Cyber Resilience / SBRI / NCSC-DSTL-aligned calls  
**Lead framing:** Offline cyber-incident evidence integrity, deterministic replay, and selective disclosure without publishing a transaction graph.

---

## Problem Statement

Cyber-incident records are routinely lost, altered, or selectively disclosed under legal, regulatory, or adversarial pressure. The root cause is architectural: existing audit-log systems either (a) expose a full transaction graph — revealing sensitive operational timing and identity patterns — or (b) require a trusted central authority to attest log integrity. Neither property survives a post-incident adversarial review or cross-jurisdictional legal challenge.

The consequences are measurable:

- Incident evidence is challenged in legal proceedings because the provenance chain is unverifiable by independent parties.
- Regulators cannot distinguish between a genuine gap in logs and a deliberate deletion.
- Operators cannot share evidence selectively (e.g., with an insurer or regulator) without revealing operationally sensitive information about unrelated incidents.

---

## Proposed Solution

QASH demonstrates a cyber-resilience substrate that separates the **commitment transcript** from the **incident body**. Operators append cryptographic commitments to a local offline write-ahead log. The commitment-only public transcript — a deterministically replayable sequence of hashes — can be verified by any independent party without network access, without a trusted intermediary, and without revealing any incident content.

Key properties that differentiate QASH from existing solutions:

1. **No transaction-graph exposure.** The public transcript contains commitment roots only. Incident body, timing, operator identity, and record count correlations cannot be recovered from it.

2. **Offline-first architecture.** The full pipeline (issue → commit → export → replay → disclose) operates without network connectivity. Designed for air-gapped operational environments and intermittently-connected field deployments.

3. **Cross-platform determinism.** SHA3-256 commitment roots are identical across x86-64, AArch64, and RISC-V — verified by CI on every commit. Independent verifiers do not need the same hardware as the issuer.

4. **Selective disclosure.** A single incident receipt can be disclosed to an authorised party (insurer, regulator, legal counsel) by receipt ID without revealing any other record.

5. **Formal assurance path.** The consensus core (Domain A) is `no_std`, no-alloc, and formally proven for stability and absorbing-halt safety properties via Coq. This is the foundation for future certification claims.

---

## Innovation

The novelty is the combination of:

- A domain-separated commitment architecture that enforces a hard boundary between the public transcript (Domain B) and the consensus state machine (Domain A).
- Cross-ISA determinism proven by CI rather than claimed by specification alone.
- An offline-first design that does not require a blockchain, distributed ledger, or trusted timestamping authority.

No existing commercial offering combines all four properties (offline operation, no graph exposure, selective disclosure, cross-platform replay independence) in a single verifiable system.

---

## Market and Application

**Primary market:** UK public sector cyber resilience — NHS Digital, NCSC-advised operators, MoD-adjacent supply chain.

**Secondary market:** Critical national infrastructure operators with air-gapped or intermittently-connected environments (energy, water, transport).

**Tertiary market:** Regulated financial sector incident reporting (FCA/PRA cyber resilience requirements).

Do not frame as: payment, settlement, token, coin, production identity, or distributed ledger for financial transactions. The technology is a cyber-resilience evidence layer, not a financial protocol.

---

## Current Status and Evidence

TRL 3–4. A working demonstrator is CI-verified on every commit:

- `bash scripts/run_mvp_demo.sh --clean` runs end-to-end in < 2 seconds.
- Replay root is stable across two sequential runs (determinism test in CI).
- No private payload appears in any public artifact (privacy boundary test in CI).
- Cross-platform build verified on x86-64, AArch64, and RISC-V on every PR.

Pilot package: `docs/pilot/pilot_package.md`. Evidence manifest: `docs/mvp/pilot_evidence_manifest.md`.

---

## Funding Ask

**Phase 1 (this application):** £[AMOUNT] over [DURATION] to complete:
- Formal partner pilot with [1–3] public sector operators.
- TRL 5 evidence matrix (hardware attestation integration, production-grade signature verification).
- Regulatory mapping (NIS2, DORA, UK NCSC Cyber Essentials Plus).
- Production-grade post-quantum signature verification (Dilithium5/SLH-DSA).

**Phase 2 (follow-on):** Multi-operator transcript merging and cross-operator audit trail replay.

---

## Claim Boundary

All claims made in this application are governed by `docs/mvp/claims_register.md`. No production deployment, genesis-admitted validator, production ZK proof, or production hardware attestation is claimed. The demonstrator is a TRL 3–4 evidence system only.
