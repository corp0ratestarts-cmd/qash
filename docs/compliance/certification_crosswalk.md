# Certification Crosswalk (FIPS 140-3 / CC-style / Internal Evidence)

This crosswalk maps QASH compliance controls to: (a) relevant FIPS 140-3 themes,
(b) Common Criteria-style ADV/ATE/ALC evidence analogs, and (c) internal proof/evidence artifacts.

| Control objective | FIPS 140-3 relevant sections (theme-level) | CC-style evidence analog | Internal proof/evidence artifacts |
|---|---|---|---|
| Algorithm correctness and determinism | Approved mode/algorithm correctness, self-test expectations | ADV_FSP (functional specification), ATE_FUN (functional testing) | `tests/vectors/cascade_kat.json`, `crates/consensus/tests/cascade_kat.rs`, CI KAT artifacts |
| DRBG health and startup assurance | Power-up and conditional self-tests, entropy/DRBG health requirements | ADV_TDS (design), ATE_COV (test coverage), ALC_CMC (configuration mgmt) | DRBG self-test logs from CI + platform test outputs (artifact bundle) |
| Build integrity and provenance | Module integrity and lifecycle assurance themes | ALC_CMS (CM scope), ALC_DEL (delivery), ALC_LCD (lifecycle definition) | Reproducible-build attestations, provenance attestations, `scripts/attest_release.sh`, `artifacts/attestations/` |
| Dependency transparency and vulnerability posture | Operational security assurance and approved operational environment themes | ALC_FLR (flaw remediation), ATE_IND (independent testing) | CycloneDX SBOM, `cargo-audit` report, `cargo-deny` report, remediation tracker |
| Formal assurance traceability | Security policy and design assurance documentation themes | ADV_RCR (correspondence), ADV_IMP (implementation representation), ATE_DPT (depth) | `proofs/COVERAGE.md`, `proofs/STATUS.md`, proof-hash manifests, refinement docs |
| Release gate completeness | Approved-mode operational controls and release process assurance themes | ALC_CMC/ALC_CMS/ALC_DEL package consistency | `artifacts/compliance/<tag>/index.json` complete evidence index + release gate check |

## Internal evidence index contract

Each release tag must publish a machine-readable index:

- Path: `artifacts/compliance/<tag>/index.json`
- Contents:
  - Commit SHA and tag.
  - Artifact URIs/digests for each required evidence class.
  - Verification status and timestamp.

This index is the canonical bridge between certification narratives and concrete CI artifacts.
