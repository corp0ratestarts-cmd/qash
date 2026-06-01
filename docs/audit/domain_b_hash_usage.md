# Domain B Hash Usage Audit

## Claim boundary

This document classifies Domain B hash uses. It does not alter Domain A, genesis
constants, QASH-CASCADE-7, GRC-7-7-v2, FIPS POST, CAVP/ACVP evidence, or sovereign
suite definitions.

All migrated sites use "independent dual-root all-of verification." No migrated site
claims FIPS validation, CAVP/ACVP status, or external certification.

## Categories

### standards-retained

Used where a named standards-aligned construction requires the primitive. These must
not be migrated.

Examples:
- FipsDrbg (HMAC-DRBG / SHA-256)
- HMAC-DRBG (NIST SP 800-90A)
- SHA-256 / SHA-3 power-on self-test (POST)
- CAVP / ACVP-style evidence primitives

### compact-hedged

Used where a compact 32-byte QASH-specific tag is sufficient and all-of verification
is not necessary.

Examples:
- `dual_hash_32` for non-critical compact Domain B tags

### all-of migrated

Used where an evidence object must verify both roots independently.

Examples:
- `ShredKeyEvidence.evidence_root_pair`
- Receipt evidence root pair
- Attestation transcript root pair
- Clone/export manifest root pair
- Evidence bundle root pair

### not applicable

Do not migrate these.

Examples:
- Domain A (crates/consensus/)
- QASH-CASCADE-7
- GRC-7-7-v2
- GENESIS_CONSTANTS.toml
- SuiteGuomi
- SuiteKorea
- X-Wing standards-named output

### dead code / remove later

None identified at this time.

---

## Do-not-migrate list

The following must not be migrated to all-of roots:

- Domain A state roots (`crates/consensus/`)
- QASH-CASCADE-7
- GRC-7-7-v2
- `GENESIS_CONSTANTS.toml`
- FipsDrbg
- HMAC-DRBG
- FIPS POST
- CAVP / ACVP tests
- X-Wing standards-named output
- SuiteGuomi
- SuiteKorea
- per-transaction hot-path admission
- per-packet transport integrity
- per-log-line hashing
- per-chunk clone transport hashing

---

## Classification table

| Use site | Path | Category | Rationale | Migration status |
|---|---|---|---|---|
| FipsDrbg | `crates/pal/src/crypto/drbg.rs` | standards-retained | HMAC-DRBG/SHA-256 standards-aligned path | retained |
| FIPS POST | `crates/pal/src/crypto/post.rs` | standards-retained | POST/KAT evidence for named algorithms | retained |
| `dual_hash_32` | `crates/pal/src/crypto/dual_hash.rs` | compact-hedged | compact QASH-specific Domain B tag | retained |
| `allof_hash_pair_32` | `crates/pal/src/crypto/dual_hash.rs` | all-of migrated | API for independent dual-root all-of verification | implemented |
| `ShredKeyEvidence.evidence_root_pair` | `crates/pal/src/privacy/erasure.rs` | all-of migrated | erasure evidence record — both arms must verify | migrated |
| receipt evidence root pair | `crates/pal/src/receipt.rs` | all-of migrated | receipt evidence metadata binding only; no raw ciphertext or key material | migrated |
| attestation transcript root pair | `crates/pal/src/lib.rs` (`mod attestation`) | all-of migrated | transcript binding only; does not certify TPM/TEE/HSM backend | migrated |
| clone/export manifest root pair | `crates/pal/src/clone/manifest.rs` | all-of migrated | one all-of root per package manifest; no per-chunk root | migrated |
| evidence bundle root pair | `crates/pal/src/evidence_bundle.rs` | all-of migrated | release/evidence manifest binding; no genesis constants recomputed | migrated |
| Domain A hashes | `crates/consensus/` | not applicable | consensus/genesis boundary — must not be touched | not migrated |
| SuiteGuomi / SuiteKorea | `crates/pal/src/crypto/agility.rs` | not applicable | sovereign suite identity must remain clean | not migrated |
| X-Wing | `crates/pal/src/crypto/kem.rs` | not applicable | standards-named KEM output | not migrated |
