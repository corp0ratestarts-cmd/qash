# ADR-017: QASH Sovereign Hardened Profile Design

**Status:** Accepted — Post-V1 Research Track  
**Date:** 2026-06-03  
**Authors:** Protocol team  
**Replaces:** None  
**Related:** ADR-015 (Pure QASH repo split), ADR-016 (Regulated Profile), `docs/spec/19_profile_taxonomy.md §19.1`

---

## Context

ADR-015 defined the Pure QASH Core / Regulated Profile split. The profile taxonomy
(`docs/spec/19_profile_taxonomy.md`) defines a third profile — **QASH Sovereign Hardened** —
as a deployment tier that combines the Pure QASH Core privacy model with an attested
hardware admission boundary in Domain B.

This ADR records the design intent, defers implementation to post-v1, and documents the
existing scaffolding that will serve as the SOV implementation base.

---

## Decision: Sovereign Hardened Profile Architecture

### D1 — Same privacy model as Pure QASH Core

The Sovereign Hardened Profile uses the identical observer class structure as Pure QASH Core:
Class I (public), Class II (validator), Class III (receipt holder). No Class IV observer,
no genesis-authorised disclosure key, no lawful-basis flows.

The profile distinction is exclusively in **Domain B** — the hardware admission boundary.
Domain A (consensus core) is profile-unaware by construction.

### D2 — Hardware admission boundary (post-v1)

The defining characteristic is a verifiable Domain B admission gate backed by attested hardware:

| Component | Technology | Status |
|-----------|-----------|--------|
| Confidential Computing host | Intel TDX, AMD SEV-SNP, ARM CCA | `📋 POST-V1` |
| Validator identity anchoring | TPM 2.0 / HSM | `📋 POST-V1` |
| DPU/SmartNIC admission boundary | Attested NIC | `📋 POST-V1` |
| IOMMU lockdown | Platform-specific | `📋 POST-V1` |
| Hardware-backed storage erasure | Platform-specific | `📋 POST-V1` |

The hardware attestation backend stubs exist in `crates/pal/src/hardware/` under
the `tpm2`, `tdx`, `sev-snp`, and `arm-cca` feature gates. All stubs return
`Err(NotAvailable)`. These are the SOV-1 scaffolding hooks.

### D3 — Zero-persistence with hardware assurance evidence

Sovereign Hardened adds hardware assurance evidence on top of the Pure QASH Core
zero-persistence requirement:

- Platform measurements attesting that the Domain B WAL redaction policies are enforced
  at the hardware level (not just software-asserted)
- TEE-sealed validator key material (not available without TEE attestation)
- IOMMU-enforced memory isolation between Domain A and Domain B processes

This is a strictly stronger claim than Pure QASH Core, which is software-only
zero-persistence.

### D4 — No feature flag for Domain B content

Unlike the Regulated Profile (which is isolated behind `--features regulated`),
Sovereign Hardened is a deployment configuration, not a compile-time feature. The
hardware attestation stubs are always compiled in; the distinction is whether the
runtime platform provides a real TPM/TDX/CCA backend.

A production Sovereign Hardened deployment requires:
1. A platform that presents real attestation evidence
2. A genesis constant `sovereign_hardened_required = true` that causes Domain B to
   refuse initialization if hardware attestation fails

### D5 — Genesis constant (post-v1)

A `[sovereign]` section will be added to `GENESIS_CONSTANTS.toml` for Sovereign
Hardened deployments. The default (Pure QASH Core and Regulated) has:

```toml
[sovereign]
hardware_attestation_required = false
```

A Sovereign Hardened genesis sets `hardware_attestation_required = true`.

This field does not exist in the genesis constants yet. Adding it is a post-v1 change
requiring the `[genesis-change-acknowledged]` PR protocol.

### D6 — Compliance evidence boundary

Sovereign Hardened certification evidence may include attestation reports
(platform PCR measurements, TEE quotes) proving the boundary is enforced.
These are control-level evidence (they prove implementation behavior) and are permitted
in the Sovereign Hardened evidence bundle.

Unlike Regulated Profile, Sovereign Hardened does not include user-activity evidence.
The zero-persistence claim is identical to Pure QASH Core.

---

## Deferred Implementation (SOV-1 through SOV-7)

| Task | Description | Target |
|------|-------------|--------|
| SOV-1 | This ADR | ✅ Done |
| SOV-2 | Implement TPM 2.0 backend in `crates/pal/src/hardware/tpm2.rs` | Post-v1 |
| SOV-3 | Implement Intel TDX backend in `crates/pal/src/hardware/tdx.rs` | Post-v1 |
| SOV-4 | Implement AMD SEV-SNP backend in `crates/pal/src/hardware/sev_snp.rs` | Post-v1 |
| SOV-5 | Implement ARM CCA backend in `crates/pal/src/hardware/arm_cca.rs` | Post-v1 |
| SOV-6 | Domain B attestation gate in PAL initialization path | Post-v1 |
| SOV-7 | Sovereign genesis constants and verification | Post-v1 |

---

## Consequences

**Positive:**
- Pure QASH Core and Regulated Profile are unaffected — no code change in this ADR.
- The attestation stubs in `crates/pal/src/hardware/` already provide the
  correct interface boundary for future SOV-2..5 implementations.
- Profile taxonomy is complete with all three profiles documented.

**Negative:**
- Sovereign Hardened is not available in v1.0. Any deployment claiming Sovereign Hardened
  properties must wait for SOV-2 through SOV-7.

**Deferred:**
- All of SOV-2 through SOV-7 (see table above).
- DPU/SmartNIC attestation support (beyond TPM/TEE scope of SOV-2..5).
- IOMMU lockdown documentation and verification tooling.
- Sovereign Hardened CI suite.

---

## Alternatives Considered

**Alternative A: Sovereign Hardened as a compile-time feature (like Regulated)**  
Rejected: Sovereign Hardened is a deployment-tier distinction, not a protocol distinction.
It does not change observer classes or consensus behavior. A feature flag would create
misleading signals about what is or is not enforced at compile time.

**Alternative B: Fold Sovereign Hardened into Pure QASH Core**  
Rejected: Attestation requirements are deployment-specific. Mandating them
in Pure QASH Core would prevent deployments on platforms without TEE support,
which would conflict with the authorized platform list in `GENESIS_CONSTANTS.toml`
(which includes platforms without TEE requirements).
