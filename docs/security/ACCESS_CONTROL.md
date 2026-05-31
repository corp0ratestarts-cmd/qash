# Access Control Policy

**Status**: Internal alignment  
**Date**: 2026-05-30  
**Scope**: QASH repository access control, signing policy, secrets handling, and key zeroization

> This document records internal access control practices. It does not constitute
> a SOC 2 access-control assessment, CMMC IA domain certification, or any external
> audit finding.

---

## Repository Access

| Control | Implementation | Evidence |
|---------|---------------|---------|
| Branch protection on `main` | Direct pushes blocked; all changes via PR | GitHub repository settings |
| CI required before merge | All blocking CI jobs must pass | `.github/workflows/ci.yml` (65 jobs) |
| Genesis-change guard | `[genesis-change-acknowledged]` token required in PR body for changes to genesis-locked artifacts | `scripts/check_genesis_change.sh`, `genesis-change-guard` CI job |
| Locked-artifact review | Changes to `spec/pdf/`, `proofs/COVERAGE.md`, `tests/vectors/` require CI gate pass | `slice-evidence-freshness`, `vector-integrity` CI jobs |

---

## Signing Policy

| Artifact | Signing method | Status |
|----------|---------------|--------|
| Release build manifests | Cosign keyless (GitHub OIDC, Sigstore TL) | Implementation complete (`.github/workflows/release.yml`) |
| Coq proof objects | SHA-256 hash manifest committed per release | CI: `proofs` job — `proof-hashes.txt` artifact |
| SBOM | CycloneDX JSON generated per push | CI: `sbom` job — `*.cdx.json` artifact |
| Git commits | Developer responsibility; not enforced by repo | N/A |

No private signing keys are stored in the repository. Release signing uses GitHub OIDC ephemeral credentials via Sigstore's keyless signing model.

---

## Secrets Handling

| Guideline | Rationale |
|-----------|-----------|
| No secrets in source | `scripts/audit_claim_boundary.sh` and gitleaks CI scan reject accidental commits |
| No raw key material in Domain A | `CapToken<T>` enforces Domain A/B boundary; no key bytes cross into consensus state |
| Receipt keys zeroized on drop | `ReceiptKey` derives `ZeroizeOnDrop`; `shred_key()` consumes by value |
| DRBG seeded from OS entropy | `FipsDrbg` uses `getrandom` (feature `std`) for seed; never user-supplied in production |
| No API keys or credentials in workflows | GitHub Actions use OIDC tokens; no long-lived secrets in workflow files |

---

## Key Zeroization

Domain B cryptographic key material uses `ZeroizeOnDrop` throughout:

| Type | Module | Zeroization |
|------|--------|-------------|
| `ReceiptKey` | `crates/pal/src/privacy/erasure.rs` | `ZeroizeOnDrop` on `material: [u8; 32]` |
| `MlKem768KeyPair` | `crates/pal/src/crypto/kem.rs` | Delegated to `ml-kem` crate internals |
| DRBG internal state | `crates/pal/src/crypto/drbg.rs` | `hmac-drbg` crate owns zeroization |

Domain A (`crates/consensus/`) does not handle key material. The `EpochState` struct contains only hash outputs and protocol counters — no key bytes.

---

## Capability-Based Domain Boundary

Domain A values can only observe Domain B inputs through `CapToken<T>` unwrapping (`into_inner()`). This is the sole observation path (Coq proof: `domain_crossing_is_explicit` in `proofs/capability/cap_token_schema.v`). The CI job `domain-a-tripwires` enforces this at every push.

---

## Non-Claims

- CMMC Practices IA.1.076–IA.3.083 compliance is not claimed.
- FedRAMP AC control family compliance is not claimed.
- This document is internal alignment evidence, not an external audit finding.
