# Non-Consensus Dependency Review

Internal review record. Not a certification or FIPS validation claim.

Date: 2026-06-01

---

## Purpose

Per the PR #218 follow-up plan, this document reviews non-consensus dependency
versions and classifies each as: **hold** (intentional pin, do not update),
**review-when-upstream-ready** (update blocked on upstream release), or
**safe-to-update** (no Domain A or CAVP impact; can be updated in a routine
deps PR).

Any dependency update that touches Domain A (`crates/consensus/`) or genesis
artifacts requires:
- Cascade KAT re-verification
- Genesis hash review
- GRC-7-7-v2 evidence re-run
- Explicit owner acknowledgement

---

## `crates/pal` — PAL Dependencies

| Dependency | Pinned version | Current latest | Classification | Rationale |
|---|---|---|---|---|
| `sha2` | `0.9` (`0.9.9` in lock) | `0.10.9` | **hold** | `hmac-drbg 0.3` requires `digest 0.9`; `sha2 0.9` provides it. Upgrading breaks FIPS DRBG path. Unblock only when a `hmac-drbg` release supports `digest 0.11`. |
| `hmac-drbg` | `0.3.0` | `0.3.0` | **hold** | No newer release available. Provides NIST SP 800-90A HMAC-SHA-256 DRBG. |
| `sha3` | `0.10` (`0.10.9`) | `0.11.x` | **review-when-upstream-ready** | PAL-only uses are Domain B (traits, KEM combiner, POST). An upgrade needs CAVP KAT re-run on SHA3-256 vectors. No Domain A impact (consensus pins its own `sha3 0.10`). |
| `blake3` | `=1.5.5` (exact pin) | `1.8.5` | **hold** | Exact pin required for cross-ISA determinism in Domain A cascade. PAL uses the same pin to avoid two blake3 versions. Update only via a coordinated cascade-KAT PR. |
| `subtle` | `2` (`2.6.1`) | `2.6.1` | **safe-to-update** | Constant-time helpers only; no cryptographic state. Semver-compatible minor bump safe. |
| `zeroize` | `1` (`1.8.2`) | `1.8.2` | **safe-to-update** | Memory wiping only. No cryptographic protocol impact. |
| `typenum` | `1.20.0` | `1.20.1` | **safe-to-update** | Type-level numerics; semver-compatible patch bump already in lock. |
| `lz4_flex` | `0.11` | `0.11.6` | **safe-to-update** | Compression utility for clone protocol. No crypto or Domain A impact. |
| `ml-kem` | `0.3` (optional) | `0.3.x` | **hold** | Pinned to FIPS 203 ML-KEM-768 implementation. Any update needs CAVP ML-KEM-768 KAT re-verification. |
| `sm3` | `0.4` (optional) | `0.5.x` | **review-when-upstream-ready** | PAL `suite_guomi` feature only. Upgrade requires SM3 KAT re-run. Does not touch Domain A (consensus pins `sm3 0.4` independently). |
| `getrandom` | `0.2` (optional) | `0.3.x` | **review-when-upstream-ready** | `getrandom::getrandom()` renamed to `getrandom::fill()` in 0.3. Three call sites require update. No CAVP impact; no Domain A impact. |

## `crates/consensus` — Domain A Dependencies

Domain A dependencies are **not touched in non-consensus dependency PRs**.
Any update to the following requires a dedicated PR with cascade-KAT evidence:

| Dependency | Pinned version | Notes |
|---|---|---|
| `sha3` | `0.10.8` | Domain A cascade L1; cascade KAT required for any update |
| `blake3` | `=1.5.5` | Domain A cascade L2; cross-ISA determinism; exact pin |
| `tiny-keccak` | `2` | Domain A cascade L3 (KangarooTwelve) |
| `sm3` | `0.4` | Domain A cascade L4 |
| `streebog` | `0.10` | Domain A cascade L5 |
| `kupyna` | `0.1.0` | Domain A cascade L6 |

---

## Recommended next actions

1. **Immediately safe:** Bump `subtle`, `zeroize`, `typenum`, `lz4_flex` patch
   versions in a single routine deps PR (no CAVP re-run needed).

2. **Blocked on upstream:** Track `hmac-drbg` for a `digest 0.11`-compatible
   release. When available, coordinate `sha2` + `hmac-drbg` upgrade with
   CAVP HMAC-SHA-256 re-verification.

3. **Requires KAT re-run:** `sha3` (PAL-only), `sm3` (PAL-only), `getrandom`
   — bundle these into a single "PAL non-FIPS-path deps" PR when ready.
   Confirm Domain A crates are NOT affected before merging.

4. **Do not touch:** All Domain A dependency versions, `blake3` pin,
   `ml-kem`, `sha2`, `hmac-drbg` until upstream releases align.

---

## PR #217 status

PR #217 (`deps: unify RustCrypto digest stack to 0.11 and getrandom to 0.3`)
is open as a draft and proposes upgrading `sha3 0.10 → 0.11`, `sm3 0.4 → 0.5`,
`streebog 0.10 → 0.11`, and `getrandom 0.2 → 0.3`.

These updates are PAL-scoped and include CAVP KAT re-verification. PR #217
may proceed after this review document is merged as the authoritative
classification of which updates are safe.
