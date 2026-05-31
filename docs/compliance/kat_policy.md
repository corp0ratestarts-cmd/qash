# QASH Known-Answer Test (KAT) Coverage Policy

Internal self-test policy. Not a certification or FIPS validation claim.

Version: 1.0  
Date: 2026-05-30

---

## 1. Policy

Every enabled cryptographic primitive used in Domain A or Domain B must have at
least one known-answer test (KAT) that runs in CI. A KAT is a deterministic
test with a fixed input and expected output derived from a normative source
(NIST test vectors, published standard, or independently computed reference).

KATs serve three purposes:
1. **Regression detection**: catches accidental code or dependency changes that
   alter cryptographic output.
2. **Cross-ISA identity evidence**: confirms the primitive produces identical
   output on all authorized ISAs (x86_64, aarch64, riscv64gc).
3. **AX-2 empirical support**: provides the "CI vector witnesses" that support
   the compiler-correctness axiom.

KATs are described as **internal self-tests**, not FIPS CAVP certificates.
Passing a KAT is not equivalent to CMVP validation.

---

## 2. Coverage Table

| Primitive | CI job | Test function / vector file | Normative source | Status |
|-----------|--------|----------------------------|------------------|--------|
| SHA3-256 | `cavp-kat` | `hash::tests::cavp_sha3_256` | NIST FIPS 202 byte-oriented test vectors | ✅ pass |
| HMAC-SHA-256 (DRBG) | `cavp-kat` | `crypto::drbg::tests::cavp_hmac_sha256` | RFC 4231 / NIST SP 800-198 | ✅ pass |
| ML-KEM-768 | `cavp-kat` | `crypto::kem::tests::cavp_ml_kem_768` | NIST FIPS 203 | ✅ pass |
| Plonky3 FRI-STARK (two-layer recursion) | `cavp-kat` | `two_layer_recursion_corpus_kat_commitment`, `two_layer_pipeline_e2e_fibonacci` | QASH PR#93 corpus profile | ✅ pass |
| QASH-CASCADE-7 (all 7 L1 primitives) | `test-determinism` | `tests/vectors/cascade_kat.json` | Internally computed; cross-ISA verified | ✅ pass |
| State-root commitment (H_domain + Encode_for_commitment) | `test-determinism` | `tests/vectors/vectors.v1.json` | ADR-003; code-derived (PDF-golden pending Phase 1-D) | ✅ pass (provisional) |
| SM3-256 (state root) | `cavp-kat` | `hash::tests::cavp_sha3_256` (via H_domain wrapper) | NIST (indirect) | ✅ pass |
| Constant-time operations | `cavp-kat` | `constant_time_audit` | Internal audit harness | ✅ pass |

---

## 3. KAT source hierarchy

| Priority | Source type | Description |
|----------|-------------|-------------|
| 1 (highest) | NIST CAVP / FIPS test vectors | Authoritative; byte-level match required |
| 2 | Published RFC / standard test vectors | Authoritative; byte-level match required |
| 3 | Cross-implementation (two independent implementations agree) | Strong; both must produce the same output |
| 4 | Internally derived, cross-ISA verified | Moderate; confirmed by multi-compiler differential CI |
| 5 (lowest) | Code-derived, single-ISA | Weakest; acceptable only as temporary scaffold pending stronger source |

State-root vectors are currently level 5 (code-derived). They will be upgraded to level 2 or 3
when Phase 1-D (manual PDF traceability verification) is complete.

---

## 4. Golden vector lock

`tests/vectors/vectors.v1.json` is locked by a SHA-256 hash recorded in
`tests/vectors/vector-hashes.txt`. The `vector-integrity` CI job fails if the
file changes without a corresponding ADR-003 version bump. This ensures KAT
vectors cannot drift silently from a code change.

---

## 5. Adding a new KAT

When introducing a new cryptographic primitive or changing an existing one:

1. Identify the normative source (NIST, RFC, etc.) or compute the reference
   output on a verified implementation.
2. Add a test function in the relevant crate under `#[test]` or `#[cfg(test)]`.
3. Add the test to the `cavp-kat` CI job in `.github/workflows/ci.yml` if it
   targets a standard primitive.
4. If the new primitive affects `tests/vectors/vectors.v1.json`, regenerate the
   file and update `tests/vectors/vector-hashes.txt` along with a version bump
   to `docs/adr/ADR-003-state-root-and-encoding.md`.
5. Update `docs/compliance/internal_crypto_evidence_matrix.md` KAT status column.

---

## 6. Internal QASH unit tests vs CAVP/ACVP tests

Some Domain B modules contain unit tests that verify determinism and domain separation but are **not** CAVP/ACVP/FIPS evidence. These tests use internally computed fixtures rather than normative standard vectors.

Examples of internal (non-CAVP) test names:

- `dual_hash_32_is_deterministic`
- `dual_hash_64_is_deterministic`
- `context_separation_changes_output`
- `salt_separation_changes_output`
- `data_separation_changes_output`
- `frame_encoding_is_unambiguous`
- `pair_root_verification_requires_sha3_root`
- `pair_root_verification_requires_blake3_root`
- `pair_root_verification_accepts_exact_match`
- `xor_output_differs_from_each_arm_for_fixture`

These tests run in the standard `cargo test` suite but are **not** added to the `cavp-kat` CI job and are **not** presented as certification evidence. They cover only QASH-internal Domain B constructions (currently `crates/pal/src/crypto/dual_hash.rs`).

---

## 7. What KATs do not provide

- FIPS CAVP certification (requires NIST submission)
- Proof of correct implementation against the full standard (only the tested
  vectors are verified; edge cases require additional testing)
- Side-channel resistance evidence (covered separately by `constant_time_audit`
  and the `sca-hardened` feature flag tests)
