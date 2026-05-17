# Audit Report — CI and Integrated Fuzzing

Date: 2026-05-17

## Scope
- GitHub Actions workflow integrity.
- Integrated fuzzing readiness in CI.

## Findings

### 1) CI workflow structural defect (high)
The main workflow `.github/workflows/ci.yml` had an unlabeled job block (a second `runs-on` + `steps` section directly after `supply-chain`) which makes the workflow invalid YAML for Actions job schema.

**Impact**
- CI can fail to load/execute reliably.
- Any newly added fuzz workflow would appear broken or "not integrating" when core CI is already malformed.

**Fix**
- Restored the missing job key as `proofs:` so the block is a valid job.

### 2) Integrated fuzzing not configured (high)
No fuzz harness or fuzz CI existed in repository at audit time (`cargo-fuzz`/`fuzz/` absent, no fuzz workflow).

**Impact**
- No continuous fuzz execution on PRs or manual dispatch.
- Security regressions in parser/transition logic less likely to be caught early.

**Fix introduced in this PR**
- Added `.github/workflows/fuzz.yml` to run `cargo-fuzz` on nightly.
- Added explicit preflight checks:
  - `fuzz/Cargo.toml` must exist.
  - `cargo fuzz list` must return targets.
- Added a smoke run for `tx_decode` with `-max_total_time=30` seconds.

## Required follow-up for maintainers
To make the fuzz workflow pass, scaffold fuzz targets:

1. Initialize harness:
   - `cargo install cargo-fuzz --locked`
   - `cargo fuzz init`
2. Add target:
   - `cargo fuzz add tx_decode`
3. Implement `tx_decode` target against the most risk-prone parsing/transition entrypoints.
4. Commit `fuzz/` directory.

Once committed, the new fuzz CI job will execute automatically.
