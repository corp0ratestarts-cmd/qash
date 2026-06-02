# Domain B Stub Register

**Date:** 2026-06-01
**Status:** Wave 3 (PR #232) — authoritative stub inventory for v1.0 genesis-candidate.

This register lists every stub, scaffold, or demo-only implementation in the Domain B
code surface (`crates/pal/`, `src/hardware/`, and Domain B paths in `src/`). Each entry
has a v1.0 disposition.

---

## Disposition taxonomy

| Class | Meaning |
|-------|---------|
| `implemented-v1` | Functionally complete for v1.0; production-ready. |
| `demo-only` | Works but uses simplified logic; must not be used in production. Gated by a feature flag or clearly named. |
| `interface-only` | Correct type surface; backend not wired; returns `NotAvailable` or equivalent. Safe to deploy (fails closed). |
| `post-v1` | Explicitly deferred past genesis lock; no v1.0 claim. |

---

## Hardware attestation backends (`src/hardware/`)

| Path | Name | Disposition | Notes |
|------|------|-------------|-------|
| `src/hardware/tpm2.rs` | TPM 2.0 attestation | `post-v1` | All methods return `Err(NotAvailable)`. Requires `/dev/tpm0`/`/dev/tpmrm0` and `tss-esapi` wiring. Gated by `tpm2` feature. |
| `src/hardware/tdx.rs` | Intel TDX | `post-v1` | All methods return `Err(NotAvailable)`. Requires kernel ≥5.19 and TDX hardware. Gated by `tdx` feature. |
| `src/hardware/arm_cca.rs` | ARM CCA | `post-v1` | All methods return `Err(NotAvailable)`. Requires ARMv9 CCA hardware with RMM. Gated by `arm-cca` feature. |
| `src/hardware/sev_snp.rs` | AMD SEV-SNP | `post-v1` | All methods return `Err(NotAvailable)`. Requires EPYC 3rd-Gen+ and kernel ≥5.19. Gated by `sev-snp` feature. |
| `src/hardware/acceleration.rs` | Software-only field acceleration | `interface-only` | Returns `NotImplemented` for accelerated field ops; fallback to software is automatic. |
| `src/hardware/power_management.rs` | Power state manager | `implemented-v1` | In-memory Domain B operational state recorder; does not control hardware. |

---

## Clone transport stubs (`crates/pal/src/clone/transport/stubs.rs`)

Six transports implementing `CloneTransport` with correct MTU and name but returning
`TransportError::NotAvailable` for send/recv until platform HAL is wired. The interface
is stable and production-ready; only the hardware integration layer is missing.

| Transport | MTU | Disposition | Notes |
|-----------|-----|-------------|-------|
| `QrTransport` | 2048 | `interface-only` | QR-code-based clone transport (§10.1) |
| `NfcTransport` | 512 | `interface-only` | NFC-based clone transport (§10.1) |
| `BleTransport` | 512 | `interface-only` | BLE dual-role clone transport (§10.1) |
| `WifiDirectTransport` | 65536 | `interface-only` | Wi-Fi Direct clone transport (§10.1) |
| `LoRaTransport` | 255 | `interface-only` | LoRa low-bandwidth clone transport (§10.1) |
| `UltrasonicTransport` | 255 | `interface-only` | Ultrasonic experimental transport (§10.1); lowest-priority carrier |

---

## Threshold signing (`crates/pal/src/threshold/talus.rs`)

| Path | Disposition | Notes |
|------|-------------|-------|
| `crates/pal/src/threshold/talus.rs` | `demo-only` | TALUS-style t-of-n threshold ML-DSA signing. Type scaffolding only; `combine_shares()` uses XOR placeholder. Full MPC (secure channels, real Shamir/Lagrange) is not implemented. Gated by `threshold-signing` feature. Must not be used in production. |

---

## Rowhammer hardening (`crates/pal/src/hardening.rs`)

| Path | Disposition | Notes |
|------|-------------|-------|
| `crates/pal/src/hardening.rs` | `interface-only` | CLFLUSH-based row refresh on x86_64; zero-cost no-op on other architectures. Gated by `hardened` feature. Interface is correct; real rowhammer mitigation requires platform integration. Documented in deployment guide. |

---

## PQC crypto-agility driver (`crates/pal/src/crypto/agility.rs`)

| Path | Disposition | Notes |
|------|-------------|-------|
| `crates/pal/src/crypto/agility.rs` | `interface-only` | NIST suite migration gate: Dilithium5 → SLH-DSA-SHA3-256 at epoch 10000. Suite selection logic is correct; actual signing/verification drivers are not yet wired. `PQC_AGILITY_EPOCH` constant is authoritative. |

---

## Bitsliced NTT / SCA hardening (`crates/pal/src/signing/bitsliced_ntt.rs`)

| Path | Disposition | Notes |
|------|-------------|-------|
| `crates/pal/src/signing/bitsliced_ntt.rs` | `interface-only` | Side-channel-aware NTT stub for non-AVX2 / non-x86_64 platforms. Reference NTT is identity transform under `sca-hardened` feature; full circuit-based NTT with zeta values is not implemented. |

---

## Plonky3 FRI-STARK verifier (`crates/pal/src/crypto/plonky3_backend.rs`)

| Path | Disposition | Notes |
|------|-------------|-------|
| `crates/pal/src/crypto/` (plonky3 feature) | `post-v1` | Production FRI-STARK verifier. Interface-only for v1.0; `plonky3` feature wires the p3-* crate stack. Full verifier implementation scheduled post-genesis. |

---

## Receipt encryption (historical)

| Path | Disposition | Notes |
|------|-------------|-------|
| `crates/pal/src/receipt.rs` (XOR path) | **deleted** | The XOR placeholder was replaced by ChaCha20-Poly1305 AEAD in Wave 3 (PR #231). No demo-only encryption path remains. |

---

## Summary

| Disposition | Count |
|-------------|-------|
| `implemented-v1` | 1 |
| `demo-only` | 1 |
| `interface-only` | 11 |
| `post-v1` | 5 |
| **deleted** (stubs removed) | 1 |

No `interface-only` or `post-v1` stub is reachable from the Domain A state-root path.
All `post-v1` items are gated by feature flags and fail closed. The one `demo-only`
item (`threshold-signing`) is explicitly feature-gated and documented as not for
production use.
