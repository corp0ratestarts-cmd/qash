# v1.0 Domain B Backend Boundary

**Date:** 2026-06-03 (updated: QASH-7 — references ADR-016 through ADR-019)
**Status:** Wave 3 (PR #232) + QASH-3..6 (PRs #239–241) — authoritative backend boundary classification for genesis-candidate.

This document classifies every Domain B backend by its v1.0 status and states the
rationale for items that are excluded from the genesis-lock scope.

See `docs/audit/domain_b_stub_register.md` for the full stub inventory with paths.

---

## Classification

### Implemented for v1.0

| Component | Path | Evidence |
|-----------|------|----------|
| Power state manager | `src/hardware/power_management.rs` | In-memory, no hardware dep; fully exercised by tests |
| Receipt encryption (AEAD) | `crates/pal/src/receipt.rs` | ChaCha20-Poly1305; 18 tests pass (tamper, wrong-key, nonce uniqueness) |
| Viewing key derivation | `crates/pal/src/receipt.rs` | SHA3-256 KDF; deterministic; tested |
| WAL zero-persistence | `crates/pal/src/zero_wal.rs` | Tested by zero-persistence integration suite |
| Dual-hash evidence | `crates/pal/src/crypto/dual_hash.rs` | SHA3-512+BLAKE3 all-of pair; CAVP KAT-verified |
| PAL trait stubs (`Time`, `Net`, `Attest`, `Halt`) | `crates/pal/src/*.rs` | Interface-complete; `hosted::Host` stub wires them for CI |

### Demo-only (not for production; feature-gated)

| Component | Feature gate | Limitation |
|-----------|-------------|------------|
| Threshold signing (TALUS) | `threshold-signing` | `combine_shares()` uses XOR placeholder; real Shamir/Lagrange not implemented. Do not deploy. |

### Interface-only (fails closed; safe to deploy without wiring)

These components have correct type signatures and fail closed (`NotAvailable`,
`NotImplemented`, or equivalent) when the hardware or backend is not present.
They are not production backends; they are correct stubs.

| Component | Feature gate | v1.0 Action |
|-----------|-------------|-------------|
| Clone transports (QR, NFC, BLE, WiFi-Direct, LoRa, Ultrasonic) | `clone-transport` (implied by `CloneTransport` trait) | Interface-only. Hardware integration post-v1. |
| Rowhammer hardening | `hardened` | Interface-only. Deployment guide documents integration requirements. |
| PQC crypto-agility driver | — (always compiled) | Suite selection correct. Signing drivers not wired until PQC migration epoch. |
| Bitsliced NTT (SCA) | `sca-hardened` | Identity transform placeholder. Full circuit post-v1. |
| Software acceleration fallback | — | Returns `NotImplemented`; automatic software fallback. |

### Post-v1 (explicitly out of scope; gated by feature flags)

| Component | Feature gate | Rationale |
|-----------|-------------|------------|
| TPM 2.0 attestation | `tpm2` | Requires production TPM hardware and tss-esapi driver. No genesis-lock claim. |
| Intel TDX | `tdx` | Requires kernel ≥5.19 and TDX-capable hardware. No genesis-lock claim. |
| ARM CCA | `arm-cca` | Requires ARMv9 CCA hardware with RMM. No genesis-lock claim. |
| AMD SEV-SNP | `sev-snp` | Requires EPYC 3rd-Gen+ and kernel ≥5.19. No genesis-lock claim. |
| Plonky3 FRI-STARK verifier | `plonky3` | Production ZK verifier deferred post-genesis. Interface-only for v1.0. |

---

## Domain A isolation guarantee

No backend in this document influences Domain A state-root computation. All Domain B
components are separated by the `CapToken` type boundary (`crates/consensus/src/domain.rs`).
Cross-domain contamination is verified by the Domain A/B boundary audit
(`scripts/audit_domain_boundary_full.sh`) which runs as a blocking CI check.

---

## What the genesis lock claims (and does not claim)

The v1.0 genesis-candidate **claims**:
- Domain A consensus (transition, Lyapunov, encoding, halt) is formally verified (TH-1 through TH-8).
- The state-root commitment path (SHA3-256 + SM3-256, folded) is deterministic and collision-resistant under AX-3.
- Receipt encryption uses ChaCha20-Poly1305 AEAD with domain-separated nonces.
- All post-v1 and interface-only stubs fail closed and are feature-gated.

The v1.0 genesis-candidate **does not claim**:
- Production hardware attestation (TPM, TDX, CCA, SEV-SNP).
- Production threshold signing.
- Production clone transport hardware integration.
- ZK proof verification (Plonky3).
- Full PQC migration (Dilithium5 → SLH-DSA at epoch 10000 is defined but not activated in v1.0).

---

## References

- `docs/audit/domain_b_stub_register.md` — full path-level stub inventory
- `docs/release/v1_axiom_boundary.md` — proof-corpus axiom classification
- `docs/release/post_allof_baseline.md` — items deferred to post-genesis
- `docs/release/genesis_candidate_gate.md` — structured gate evaluation (GEN-1..8)
- `docs/adr/ADR-013_v1_backend_boundary.md` — original v1 backend boundary ADR
- `docs/adr/ADR-016-regulated-profile-design.md` — Regulated Profile (Class IV, disclosure key)
- `docs/adr/ADR-017-sovereign-hardened-profile.md` — Sovereign Hardened Profile (post-v1)
- `docs/adr/ADR-018-production-networking.md` — clone transport gap (NET-1..7)
- `docs/adr/ADR-019-zk-threshold-gap.md` — ZK/threshold signing gap (ZK-1..4, THR-1..5)
- `docs/adr/ADR-005-rust-toolchain-version.md` — toolchain pin
- `GENESIS_CONSTANTS.toml` — authoritative genesis parameters
