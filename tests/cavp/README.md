# CAVP-Style KAT Vector Fixtures

Internal known-answer test vector fixtures following ACVP JSON structure.

**Status**: Implementation complete / self-tested. Not an NIST ACVP submission.

## Files

| File | Algorithm | Source |
|------|-----------|--------|
| `sha3_256.json` | SHA3-256 | NIST FIPS 202 Appendix A/B |
| `hmac_sha256.json` | HMAC-SHA-256 | RFC 4231 §4 / NIST SP 800-198 |
| `ml_kem_768.json` | ML-KEM-768 | Internal deterministic vectors, `ml-kem` crate v0.3 |

## Format

Each file follows a subset of the ACVP JSON structure:
- `algorithm`, `revision`, `source` — identifies the standard
- `testGroups[].tests[]` — individual test cases with hex-encoded inputs/outputs
- All byte fields are lowercase hex, no `0x` prefix

## CI coverage

These fixtures are evidence artifacts that mirror the hardcoded in-code KAT tests:
- `cargo test -p qash-consensus --no-default-features -- hash::tests::cavp_sha3_256`
- `cargo test -p qash-pal -- crypto::drbg::tests::cavp_hmac_sha256`
- `cargo test -p qash-pal --features pqc -- crypto::kem::tests::cavp_ml_kem_768`

## Non-claims

- These vectors have not been submitted to the NIST ACVP server.
- No CMVP certificate or formal NIST validation exists.
- ML-KEM-768 vectors were captured from `ml-kem` crate v0.3 and verified
  for cross-platform determinism (x86_64 / aarch64) via the platform-determinism CI job.
  They are NOT official NIST ACVP vectors.
