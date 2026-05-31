# Internal Controls Matrix

**Date:** 2026-05-30  
**Status:** Internal controls documentation (not SOC 2 / CMMC / FedRAMP certified)

---

**Disclaimer:** This matrix documents internal controls and evidence. It does not constitute a SOC 2 report, CMMC assessment, FedRAMP authorization, or any external certification.

---

## Controls

| Control | Category | Evidence artifact | CI/repo gate |
|---------|----------|-------------------|--------------|
| Access control — branch protection, signed commits | Security | GitHub branch protection rules | Enforced by repository settings |
| Change management — PR review + CI gates | Process | `.github/workflows/ci.yml` (63 blocking jobs) | All PRs require CI green before merge |
| Genesis change guard — special token required for genesis modifications | Process | `scripts/check_genesis_change.sh`, `[genesis-change-acknowledged]` PR token | `genesis-change-guard` CI job |
| Supply chain — dependency vulnerability scanning | Security | `deny.toml`, `osv-ignore.toml` | `supply-chain` and `osv-scan` blocking CI jobs |
| Vulnerability management | Security | `SECURITY.md`, `osv-ignore.toml` | OSV scan CI job, cargo-deny |
| Code quality — Domain A boundary enforcement | Technical | `scripts/audit_domain_boundary_full.sh` | `domain-a-tripwires`, `domain-a-full-boundary` blocking CI jobs |
| Cryptographic key handling | Technical | `crates/pal/src/privacy/erasure.rs` (ZeroizeOnDrop, shred_key()) | `zero-persistence-boundary` CI job |
| Availability — reproducible build | Technical | `scripts/verify_two_stage_build.sh` | `cross-verify` CI job (aarch64 + riscv64) |
| Secret scanning | Security | GitHub advanced security | `Secret scanning` CI job |
| Formal proof coverage | Technical | `proofs/COVERAGE.md` (43 PROVED + CI-VERIFIED) | `proofs` blocking CI job |
