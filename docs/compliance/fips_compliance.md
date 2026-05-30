# FIPS 140-3 Alignment — QASH Domain B

> **Claim boundary:** This document uses "FIPS-aligned" throughout. "FIPS-aligned"
> is not "FIPS validated." FIPS 140-3 module validation requires engagement with
> a NIST-accredited CMVP lab. This document records implementation evidence
> intended to support a future CMVP submission — not a current validation claim.

This document maps FIPS 140-3 Level 3 (L3) requirements to their concrete
implementations in `qash-pal` (Domain B). Where an L3 requirement is not yet
satisfied, the gap and the planned phase are noted. Domain A (`qash-consensus`)
is proof-eligible consensus logic and is explicitly **out of scope** for FIPS
evaluation; it contains no FIPS-approved or non-approved algorithms.

### FIPS 140-3 Level 3 — requirement overview

FIPS 140-3 builds incrementally: an L3 module must satisfy all L1 and L2
requirements plus additional physical and identity-based constraints:

| Level | Key additions |
|-------|---------------|
| L1 | Approved algorithms, correct documentation, self-tests |
| L2 | Tamper-evident coatings or seals; role-based authentication |
| **L3** | **Tamper-response / zeroization on intrusion; identity-based auth; environmental failure protection (EFP/EFT)** |

The sections below map the four FIPS 140-3 SP 800-140C requirement families
(Section 4 Entropy, Section 6 Key Management, Section 9 Self-tests,
Section 10 Physical Security) to their QASH implementations.

---

## 1. Approved Algorithms (AS.01)

| Algorithm | Role | Implementation | Reference |
|-----------|------|----------------|-----------|
| ML-KEM-768 | Post-quantum KEM (CNSA 2.0) | `crates/pal/src/crypto/kem.rs` — `ml-kem` crate, feature `pqc` | FIPS 203 |
| SHA3-256 | X-Wing combiner, Domain A state-root hashing | `crates/consensus/src/hash.rs` (Domain A); X-Wing combiner (Domain B) | FIPS 202 |
| HMAC-SHA-256 DRBG | Deterministic random bit generation | `crates/pal/src/crypto/drbg.rs` — `hmac-drbg` crate | NIST SP 800-90A Rev.1 |
| SHA-256 | HMAC-DRBG internal hash | Provided by `hmac-drbg` / `sha2` crate | FIPS 180-4 |
| Dilithium5 | Primary post-quantum signature (planned) | `crates/pal/src/` (Phase 2-G extension) | FIPS 204 |
| SLH-DSA-SHA3-256 | Anchor signature (planned) | `crates/pal/src/` (Phase 2-G extension) | FIPS 205 |

---

## 2. Random Bit Generation (RBG) (AS.07 / AS.08)

**Implementation:** `crates/pal/src/crypto/drbg.rs` — `FipsDrbg`

- Algorithm: HMAC-DRBG with SHA-256 (NIST SP 800-90A Rev.1 §10.1.2)
- Entropy source: OS CSPRNG via `getrandom` (satisfies SP 800-90B when
  running on FIPS-validated hardware RNG)
- Nonce: independent entropy call per SP 800-90A §8.6.7
- Personalization string: mandatory caller-supplied context label (≤ 32 bytes)
- Reseed interval: ≤ 2²⁰ generate calls (conservative; SP 800-90A allows 2⁴⁸)
- Automatic reseed: `FipsDrbg::fill_bytes` reseeds before the interval expires

**Usage gate:** All Domain B key generation MUST use `FipsDrbg` with an
appropriate personalization string (e.g. `b"qash/pal/kem_keygen/v1"`).
Callers MUST NOT pass OS entropy directly to cryptographic functions.

---

## 3. Key Management (AS.05)

| Requirement | QASH Implementation |
|-------------|---------------------|
| Key generation | `MlKem768KeyPair::from_seed()` requires a `FipsDrbg`-generated 64-byte seed |
| Key zeroization | `ml-kem` crate zeroizes private key material on drop |
| Key separation | Domain A/B boundary enforced at compile time; consensus state never carries Domain B keys |
| Key storage | Planned: TPM-sealed storage via `Attest::tpm_quote()` PAL trait |

---

## 4. TLS and Transport Security (AS.10)

**Requirement:** All external connections MUST use TLS 1.2 or higher.

**Implementation:**
- `crates/pal/src/crypto/tls.rs` — `validate_tls_config(tls_version: Option<u16>)`:
  - Rejects `None` (no TLS) with `TlsConfigError::NoTls`
  - Rejects SSLv3 (0x0300), TLS 1.0 (0x0301), TLS 1.1 (0x0302) with
    `TlsConfigError::VersionBelowFloor`
  - Accepts TLS 1.2 (0x0303) and TLS 1.3 (0x0304)
  - Call at PAL init time before establishing any network transport
- `log_pseudonym(prefix, pk_bytes)`: derives a 16-byte truncated SHA3-256
  pseudonym for validator identities in log lines. Raw public keys, IP
  addresses, and validator IDs must NEVER appear in log output.
- Hosted PAL (`Host` in `crates/pal/src/lib.rs`) currently uses an in-process
  log rather than an external TLS channel.
- When a network transport is wired in, it MUST call `validate_tls_config`
  at initialisation and reject any configuration that returns an error.

**Validation gate:** The `cavp-kat` CI job enforces that any new Domain B
crypto primitive has a known-answer test before merge.

---

## 5. Physical Security (AS.02)

FIPS 140-3 Level 1 does not require physical security mechanisms. Higher
levels (L2+) require tamper-evident seals (L2) or tamper-response (L3).

For high-assurance deployments targeting Level 2+:
- `SoftTRR` / `CATT` kernel modules (Phase 2-O) mitigate DRAM Rowhammer
- `Hancke-Kuhn` distance-bounding stub (Phase 2-O) limits relay attacks on
  validator admission
- TPM measured boot integration via `Attest` PAL trait

---

## 6. Self-Tests (AS.09)

| Test | Trigger | Coverage |
|------|---------|----------|
| ML-KEM-768 KAT (encap/decap roundtrip) | CI (`cargo test --features pqc`) | `crypto::kem::tests::kat_encap_decap_roundtrip` |
| HMAC-DRBG determinism + personalization | CI (`cargo test`) | `crypto::drbg::tests::*` (6 tests) |
| Domain A hash KATs (SHA3-256) | CI | `hash::tests::h_domain_state_root_hello_known_vector`, `sha3_256_known_vector` |
| LSH-256/512 KATs | CI | `lsh256::tests::*`, `lsh512::tests::*` |

Power-on self-tests (POST) required by FIPS 140-3 §AS.09: planned for
Phase 2-P as part of the `cavp-kat` CI gate.

---

## 7. Logging and Audit (AS.11)

**Current:** `Host` appends canonical input records to a binary log
(`QPALOG1\0` magic, per-record magic `QPAIN1\0\0`). The log is integrity
protected by being append-only and replay-verified.

**FIPS extension (Phase 2-H):**
- Log entries MUST NOT include raw public keys or IP addresses; use hashed
  validator IDs (`H_ValidatorId(id)` via `DomainTag::ValidatorId`)
- Audit records MUST be protected against modification (planned: HMAC-SHA-256
  MAC per record keyed from TPM-sealed audit key)

---

## 8. Non-Approved Algorithm Usage

The following algorithms appear in the codebase but are **not** used in the
FIPS module boundary:

| Algorithm | Location | Classification |
|-----------|----------|----------------|
| BLAKE3 | `crates/consensus/src/cascade.rs` | Domain A — consensus only; not a FIPS boundary |
| KangarooTwelve | Planned cascade stage | Domain A — not a FIPS boundary |
| SM3 / SM4 | Planned (Phase 2-O sovereign suite) | Non-approved; gated behind `feature = "suite_guomi"` |
| LSH-256 / LSH-512 | `crates/consensus/src/lsh256.rs`, `lsh512.rs` | Domain A state-root hashing; not a FIPS boundary |

---

## 9. Crypto-Agility Traits (AS.01 extensibility)

`crates/pal/src/crypto/traits.rs` defines:

| Trait | Purpose |
|-------|---------|
| `HasherTrait` | Domain-tagged hash abstraction |
| `KemTrait` | Key encapsulation abstraction |
| `CipherTrait` | AEAD cipher abstraction (`std` only) |
| `SignatureTrait` | Digital signature abstraction (`std` only) |

Suite implementations:

| Suite | Struct | Feature flag | Hash | Notes |
|-------|--------|-------------|------|-------|
| Standard | `SuiteStandard` | (always) | SHA3-256 | FIPS-aligned baseline |
| Guomi | `SuiteGuomi` | `suite_guomi` | SM3 | GM/T sovereign standard |
| Korea | `SuiteKorea` | `suite_korea` | LSH-512/256 (KS X 3262) | Korean national standard |

**Domain A parity invariant:** toggling any suite feature must not change
`crates/consensus/` — `git diff crates/consensus/` must remain empty after
any feature toggle.

---

## 10. FIPS 140-3 Validation Roadmap

| Phase | Action |
|-------|--------|
| 2-G (done) | ML-KEM-768 KAT + HMAC-DRBG KAT in CI |
| 2-P | Add `cavp-kat` CI job (NIST ACVP schema); block PRs on failure |
| 2-P | `dudect-bencher` constant-time audit for all Domain B crypto paths |
| Post-Genesis | Engage FIPS 140-3 Level 1 CMVP lab; submit Security Policy |
| Post-Genesis | CC EAL4+ evaluation using `docs/compliance/cc_security_target.md` |
