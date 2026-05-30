# QASH Sovereign Cryptography Matrix

**Date:** 2026-05-30  
**Status:** Internal alignment (not externally validated)  
**Audience:** Compliance reviewers, jurisdiction-specific integration partners, auditors,
and regulators seeking evidence of sovereign algorithm coverage.

---

## Non-Claims Boundary

This document records implementation evidence for national / sovereign cryptographic
algorithms present in QASH. It does **not** claim that QASH holds any national
certification, KCMVP certification, Guomi certification, GOST FSTEC certification,
or equivalent regulatory approval in any jurisdiction.

The permitted claim is:

> QASH implements a set of sovereign hash primitives as Domain A cascade stages.
> Each is covered by CI-verified known-answer tests. No formal national certification
> has been obtained and none is claimed.

---

## Status Vocabulary

The following status labels are used throughout this document:

| Status label | Meaning |
|---|---|
| N/A | Not applicable to this repo |
| Internal alignment | Code/design follows the standard's approach; no external assessment |
| Implementation complete / self-tested | Implemented with CI KATs; no external validation |
| Externally certified | Formal certificate or report exists — none currently |

---

## Sovereign Algorithm Matrix

| Jurisdiction / Standard | Algorithm | Module / feature flag | KAT status | Production status | Claim boundary |
|------------------------|-----------|----------------------|------------|-------------------|----------------|
| Korea / LSH (KS X 3262) | LSH-512 | `crates/consensus/src/lsh512.rs` — no feature flag (always-on) | CI-verified (cascade KAT) | Domain A cascade primitive | Implementation complete / self-tested — KCMVP certification is not claimed |
| Korea / LSH (KS X 3262) | LSH-256 | `crates/consensus/src/cascade.rs` — no feature flag (always-on) | CI-verified (cascade KAT) | Domain A cascade primitive | Implementation complete / self-tested — KCMVP certification is not claimed |
| China / Guomi (GM/T 0004-2012) | SM3 | `crates/consensus/src/sm3.rs` + `crates/pal/` — no feature flag | CI-verified (cascade KAT + state-root KAT) | Domain A hash; Domain B opt-in | Implementation complete / self-tested — SM3 hash only; full GM/T suite (SM2, SM4) is not implemented; Guomi certification is not claimed |
| Russia / GOST (GOST R 34.11-2012) | Streebog-512 | `crates/consensus/src/streebog512.rs` — no feature flag (always-on) | CI-verified (cascade KAT) | Domain A cascade primitive | Implementation complete / self-tested — cipher suite (GOST 28147-89 / Grasshopper) is not implemented; GOST FSTEC certification is not claimed |
| Ukraine / Kupyna (DSTU 7564:2014) | Kupyna-512 | `crates/consensus/src/kupyna.rs` — no feature flag (always-on) | CI-verified (cascade KAT) | Domain A cascade primitive | Implementation complete / self-tested — no formal certification exists or is claimed |

---

## Notes

### Korea / LSH (KS X 3262)

- LSH-512 is a Domain A always-on cascade primitive (layer 7 in the seven-layer hash cascade).
- LSH-256 is used as a cascade stage within `crates/consensus/src/cascade.rs`.
- **Vector source:** The `abc` test vector used in CI is implementation-captured
  (derived from the QASH implementation itself during initial integration), not sourced
  from the official KS X 3262 KCMVP test suite.
- **KCMVP certification is not claimed.** The Korean Cryptographic Module Validation
  Programme requires formal evaluation by an accredited Korean laboratory. No such
  evaluation has been initiated.

### China / SM3 (GM/T 0004-2012)

- SHA3 is not SM3. These are distinct hash functions. QASH supports SM3 (GM/T 0004-2012);
  it does **not** implement the full GM/T cryptographic suite.
- The following GM/T components are **not** implemented: SM2 (elliptic-curve digital
  signature and key agreement), SM4 (128-bit block cipher).
- SM3 is used in Domain A (`crates/consensus/src/sm3.rs`) for the state-root hash and
  as a cascade stage, and is available in Domain B via `crates/pal/`.
- **Guomi certification is not claimed.** No GM/T certification or OSCCA evaluation has
  been initiated.

### Russia / Streebog (GOST R 34.11-2012)

- QASH implements the Streebog-512 hash function (GOST R 34.11-2012) as a Domain A
  cascade stage (`crates/consensus/src/streebog512.rs`).
- The GOST block cipher (GOST 28147-89) and its successor (Grasshopper / GOST R 34.12-2015)
  are **not** implemented.
- **No GOST FSTEC certification is claimed.** Russian Federal Service for Technical and
  Export Control certification has not been initiated.

### Ukraine / Kupyna (DSTU 7564:2014)

- QASH implements the Kupyna-512 hash function (DSTU 7564:2014) as a Domain A cascade
  stage (`crates/consensus/src/kupyna.rs`).
- **No formal certification is claimed.** No Ukrainian State Service of Special
  Communications evaluation or equivalent has been initiated.

### General note on Domain A cascade primitives

All sovereign algorithms listed in this matrix are Domain A primitives. Domain A rules
apply:

- No `unsafe`, no floating point, no `usize`/`isize` in wire-format arithmetic.
- All arithmetic is checked; overflow triggers `Halt::absorbing_reset()`.
- Replay-invariant across all authorised ISAs (x86-64, aarch64, riscv64gc).

---

## Evidence Gaps

| Gap | Description |
|-----|-------------|
| LSH KAT source | Official KS X 3262 KCMVP published test vectors not yet integrated; current vectors are implementation-captured |
| SM3 extended KAT | Only cascade-level and state-root KATs exist; OSCCA published SM3 test vectors not systematically cross-checked |
| Streebog NESSIE/GOST vectors | CI uses `streebog v0.10` crate vectors; independent GOST R 34.11-2012 official test-vector cross-check not documented |
| Kupyna extended KAT | CI uses `kupyna v0.1.0` crate vectors; DSTU 7564:2014 official test-vector cross-check not documented |

---

## Roadmap

| Phase | Action |
|-------|--------|
| 2-P | Integrate official KS X 3262 test vectors and replace implementation-captured LSH vectors |
| 2-P | Cross-check SM3 against OSCCA published test vectors; document evidence artifact |
| 2-P | Document Streebog and Kupyna vector provenance against official standards bodies |
| Post-Genesis | Assess feasibility of KCMVP, GM/T, and/or GOST FSTEC evaluation pathways |
