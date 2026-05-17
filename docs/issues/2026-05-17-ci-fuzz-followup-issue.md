# Issue: CI fuzzing/deny alignment follow-up

Date: 2026-05-17

## Summary
Follow-up issue to track remaining CI consistency risks after the workflow syntax fix and initial fuzz integration.

## Problem statement
1. Fuzz implementation mismatch: reports indicate a honggfuzz migration, but repository workflow remains cargo-fuzz based.
2. Fuzz harness expectation mismatch: workflow requires `fuzz/Cargo.toml`, but that path may be absent in some branches.
3. Supply-chain policy drift risk: workspace crates use `GPL-3.0-or-later` while `deny.toml` allowlist omits GPL entries.

## Evidence
- `.github/workflows/fuzz.yml` currently uses `cargo fuzz list` and `cargo fuzz run tx_decode`.
- `deny.toml` `[licenses].allow` list does not include GPL strings.
- Workspace crate manifests declare `license = "GPL-3.0-or-later"`.

## Impact
- CI behavior can differ from maintainer expectations (green locally vs red in GitHub Actions).
- Fuzz job can fail as configuration error rather than meaningful security signal.
- `cargo deny check` may fail for policy reasons unrelated to dependency vulnerabilities.

## Proposed resolution
1. Choose one fuzz engine (honggfuzz or cargo-fuzz) and update workflow + docs to match.
2. Add deterministic preflight checks with actionable failures for missing harness/targets.
3. Align `deny.toml` license policy with intended workspace licensing posture (or explicitly scope checks to dependencies only, if preferred).
4. Add a short CI troubleshooting section to docs that maps failing job -> remediation.

## Acceptance criteria
- Fuzz workflow implementation matches documented engine.
- Fuzz job passes on a PR with harness present and fails with clear message when absent.
- Supply-chain job passes with an intentional, documented license policy.
- New contributor can follow docs to reproduce CI checks locally.
