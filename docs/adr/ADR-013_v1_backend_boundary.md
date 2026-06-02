# ADR-013: v1.0 Backend Boundary Decision

**Date:** 2026-06-01  
**Status:** Accepted  
**Deciders:** Protocol maintainers  
**References:** Wave 3 PR #232; `docs/release/v1_domain_b_backend_boundary.md`; `docs/audit/domain_b_stub_register.md`

---

## Context

The QASH v1.0 genesis-candidate ships with a Domain B PAL layer that
contains multiple hardware and platform backends at different readiness levels.
Before the genesis lock we need an authoritative record of which backends are
in scope for v1.0, which are demo-only, and which are deferred to post-genesis.
This ADR records the decision.

---

## Decision

The v1.0 release boundary is defined across four tiers:

### Tier 1 — Implemented for v1.0 (production-eligible)

All of the following are fully exercised by the CI test suite and make no
hardware claims beyond what is present in every Linux CI runner:

| Component | Path | Evidence |
|-----------|------|---------|
| Power state manager | `src/hardware/power_management.rs` | In-memory; no hardware dep |
| Receipt encryption (AEAD) | `crates/pal/src/receipt.rs` | ChaCha20-Poly1305; 18 tests |
| Viewing key derivation | `crates/pal/src/receipt.rs` | SHA3-256 KDF; deterministic |
| WAL zero-persistence | `crates/pal/src/zero_wal.rs` | Zero-persistence integration suite |
| Dual-hash evidence | `crates/pal/src/crypto/dual_hash.rs` | CAVP KAT-verified |
| PAL trait stubs (`Time`, `Net`, `Attest`, `Halt`) | `crates/pal/src/*.rs` | `hosted::Host` wires for CI |

### Tier 2 — Demo-only (feature-gated; not for production)

| Component | Feature gate | Limitation |
|-----------|-------------|------------|
| Threshold signing (TALUS) | `threshold-signing` | `combine_shares()` uses XOR placeholder; no Shamir/Lagrange |

### Tier 3 — Interface-only (fails closed; safe to deploy without wiring)

These have correct type signatures and return `NotAvailable`/`NotImplemented`
when hardware is absent. No genesis-lock production claim is made.

| Component | Feature gate |
|-----------|-------------|
| Clone transports (QR/NFC/BLE/WiFi-Direct/LoRa/Ultrasonic) | `clone-transport` |
| Rowhammer hardening | `hardened` |
| PQC crypto-agility driver | always compiled; suite selection correct |
| Bitsliced NTT (SCA) | `sca-hardened` |
| Software acceleration fallback | — |

### Tier 4 — Post-v1 (no genesis-lock claim)

| Component | Feature gate | Requirement |
|-----------|-------------|-------------|
| TPM 2.0 attestation | `tpm2` | Production TPM hardware + tss-esapi |
| Intel TDX | `tdx` | Kernel ≥5.19, TDX-capable hardware |
| ARM CCA | `arm-cca` | ARMv9 CCA hardware with RMM |
| AMD SEV-SNP | `sev-snp` | EPYC 3rd-Gen+, kernel ≥5.19 |
| Plonky3 FRI-STARK verifier | `plonky3` | Production ZK verifier |

---

## Consequences

1. **Domain A isolation is preserved.** No Tier 2/3/4 component influences
   Domain A state-root computation. The `CapToken` type boundary in
   `crates/consensus/src/domain.rs` enforces this; the Domain A/B boundary
   audit script verifies it on every CI run.

2. **Honest scope.** The genesis-candidate does not claim production hardware
   attestation, production threshold signing, ZK proof verification, or full
   PQC migration. These are all post-genesis work items.

3. **Feature gates are enforced.** Tier 2/3 code is reachable only when the
   relevant feature flag is explicitly set; CI runs with `--no-default-features`
   and with `--features std` — neither of which activates Tier 2/3/4 gates.

4. **Stub register is the authoritative inventory.** Any addition or change to
   Tier 2/3/4 components requires an update to
   `docs/audit/domain_b_stub_register.md` as a pre-merge gate.

---

## Alternatives considered

**Alternative A — ship all backends as interface-only without classification.**
Rejected: ambiguous scope leads to inflated claims and audit confusion.

**Alternative B — defer all Domain B except PAL traits to post-genesis.**
Rejected: receipt encryption (AEAD) and WAL zero-persistence are required for
correct operation of the production hosted binary; they cannot be deferred.

**Alternative C — include TPM/TDX/SEV-SNP as "partially implemented".**
Rejected: no CI runner exercises the hardware path; the claim would be
unverifiable and misleading.
