# Compliance Stage Gates

This document defines mandatory compliance evidence gates for each delivery phase.

## Evidence classes (required at every phase)

1. Cryptographic KATs.
2. DRBG health/self-tests.
3. Reproducible build attestations.
4. SBOM + vulnerability scan.
5. Proof coverage report.

## Phase gates

## 1) Dev

**Purpose:** Continuous developer feedback and early detection.

**Minimum gate evidence:**
- Cryptographic KAT results from deterministic vector suites.
- DRBG health/self-test logs from unit/integration smoke tests.
- Draft reproducible-build attestation from CI build metadata.
- Preliminary SBOM and vulnerability scan artifacts.
- Current proof coverage snapshot.

## 2) Pre-merge

**Purpose:** Block unsafe changes before landing on protected branches.

**Required evidence:**
- Passing cryptographic KAT artifacts attached to the merge pipeline.
- DRBG health/self-test pass records (no ignored failures).
- Reproducible build attestation for merge commit.
- SBOM + vulnerability scan reports with policy evaluation.
- Proof coverage report showing no unauthorized regression.

## 3) RC (Release Candidate)

**Purpose:** Freeze candidate evidence set for release sign-off.

**Required evidence:**
- Signed cryptographic KAT bundle for release-candidate commit.
- Signed DRBG health/self-test bundle for target platforms.
- Release-candidate reproducible build attestations (builder + source digest).
- Signed SBOM + vulnerability scan package.
- Proof coverage report pinned to RC commit and proof artifact hashes.

## 4) Release

**Purpose:** Authorize tag and publication.

**Required evidence:**
- Final cryptographic KAT package for tagged commit.
- Final DRBG health/self-test package for approved targets.
- Provenance + reproducible build attestations for published binaries.
- Final SBOM + vulnerability scan package with policy disposition.
- Final proof coverage report and proof-artifact index.

**Release tag policy (mandatory):**
- No release tag may be cut unless `artifacts/compliance/<tag>/index.json` exists and indexes all required evidence classes for that exact tag.

## 5) Post-release monitoring

**Purpose:** Detect regressions and newly introduced risk after publication.

**Required evidence:**
- Scheduled re-run of cryptographic KATs on maintained release branches.
- Scheduled DRBG health/self-test telemetry review.
- Ongoing reproducible build attestation spot-checks.
- SBOM refresh + vulnerability rescan cadence with remediation tracking.
- Proof coverage drift report for any backported changes.
