# SHA-2 Usage Audit

Internal audit record. Not a certification or FIPS validation claim.

Date: 2026-06-01  
Scope: All `sha2` crate usage in the QASH workspace.

---

## Purpose

The rule for QASH is:

> Do not use SHA-2 as the **sole final** QASH-specific Domain B commitment boundary.

SHA-2 remains permitted — and required — where it is part of a named
standards-aligned construction. This document classifies every `sha2` use in
the workspace into one of three categories:

| Category | Meaning |
|---|---|
| **standards-aligned / retained** | SHA-2 is required by a named standard (FIPS, SP 800-series, RFC). Must not be removed. |
| **qash-specific / migrated** | SHA-2 was used as a sole QASH-specific commitment boundary. Has been or should be replaced with `dual_hash_32`. |
| **dead code / removed** | Unused. Should be deleted. |

---

## Audit Results

### `crates/pal/src/crypto/drbg.rs`

| Use site | Line(s) | Classification | Rationale |
|---|---|---|---|
| `use sha2::Sha256` | 15 | **standards-aligned / retained** | HMAC-DRBG per NIST SP 800-90A Rev 1 requires HMAC-SHA-256. `hmac-drbg 0.3` requires `sha2 0.9`. Cannot be removed without dropping standards alignment. |
| `use sha2::{Digest, Sha256}` in `cavp_hmac_sha256` test | 275 | **standards-aligned / retained** | CAVP HMAC-SHA-256 KAT (RFC 4231 vectors). Test is standards evidence; must stay tied to SHA-256. |

### `crates/pal/src/crypto/post.rs`

| Use site | Line(s) | Classification | Rationale |
|---|---|---|---|
| `use sha2::Digest as Sha2Digest` | 18 | **standards-aligned / retained** | FIPS 140-3 POST (Power-On Self-Test) for SHA-256 module boundary, per FIPS 180-4. Required for CMVP evidence path. |
| `sha2::Sha256::digest(b"")` in `post_sha256` | 73 | **standards-aligned / retained** | NIST FIPS 180-4 known-answer test (empty-input SHA-256 vector). Must match the standard vector byte-for-byte. |

### `src/hardware/capabilities.rs`

| Use site | Line(s) | Classification | Rationale |
|---|---|---|---|
| `is_aarch64_feature_detected!("sha2")` | 63 | **standards-aligned / retained** | This is CPU ISA feature detection (AArch64 SHA2 instruction availability), not a crate dependency. No action needed. |

---

## Summary

**No QASH-specific SHA-2 uses were found.** All `sha2` uses in the workspace
are tied to named standards (NIST SP 800-90A, FIPS 180-4) or ISA feature
detection. None require migration to `dual_hash_32`.

**`sha2 = "0.9"` must be retained** because:
- `hmac-drbg 0.3` requires `digest 0.9`, which `sha2 0.9` provides.
- No upstream `hmac-drbg` release exists for `digest 0.11`.
- Upgrading `sha2` without a compatible `hmac-drbg` would break the FIPS DRBG path.

A future `hmac-drbg` upgrade (to a version supporting `digest 0.11`) may allow
`sha2` to be updated, but that belongs in a dedicated dependency-review PR with
full CAVP re-verification.

---

## Forbidden actions (do not do in any follow-up PR)

- Do not remove `sha2` from `crates/pal/Cargo.toml`.
- Do not remove `hmac-drbg` from `crates/pal/Cargo.toml`.
- Do not replace `FipsDrbg` with `QashHedgedDrbg`.
- Do not alias `FipsDrbg` to any non-HMAC-DRBG type.
- Do not add FIPS/CAVP/ACVP labels to any `dual_hash`-based construction.
