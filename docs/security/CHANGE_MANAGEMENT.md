# Change Management Policy

**Date:** 2026-05-30

---

## Core Rules

1. **All changes via PR.** Direct pushes to `main` are prohibited; branch protection enforces this.

2. **CI must be green before merge.** All blocking CI jobs must reach `conclusion: success` before a PR may be merged.

3. **Genesis-affecting changes** require `[genesis-change-acknowledged]` in the PR description, and the `genesis-change-guard` CI job must pass.

4. **Locked artifacts** (`GENESIS_CONSTANTS.toml` and files listed in `spec/genesis-artifacts.txt`) require explicit manual reviewer confirmation before approval.

5. **Proof changes:** Any new `Axiom` added under `proofs/` must be documented in `proofs/COVERAGE.md` before merge (enforced by `check_axiom_coverage.sh`).

6. **Claim boundary:** No new overclaims are permitted (enforced by `audit_claim_boundary.sh` as a blocking CI job).

---

## Release Checklist

- [ ] `cargo fmt --all -- --check` clean
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `cargo test --workspace --no-default-features` passes
- [ ] `bash scripts/verify_genesis_hash.sh` passes
- [ ] Claim boundary scan clean (`bash scripts/audit_claim_boundary.sh`)
- [ ] OSV / supply-chain scan clean (`cargo deny check` + OSV CI job)
- [ ] Evidence bundle captured: `bash scripts/capture_pre_genesis_evidence.sh`
- [ ] **For genesis lock:** set `genesis_status = "locked"` and `deployment_authoritative = true` in `GENESIS_CONSTANTS.toml`, tag `v1.0-reference`
