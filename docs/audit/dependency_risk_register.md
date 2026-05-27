# Dependency Risk Register

**Scope:** All crates in the QASH workspace, including transitive dependencies.  
**Requirement:** Advisory findings from `cargo audit`, OSV scan, and `cargo deny` must be triaged into this register before genesis-lock. Each entry records the fields below.  
**Owner:** Repository maintainers.

---

## Register format

Each entry:

```markdown
### <crate>@<version> — <CVE/advisory ID>

**Advisory:** <CVE-YYYY-NNNNN or RUSTSEC-YYYY-NNNN or OSV-YYYY-NNNNNN>
**Dependency type:** Direct / Transitive
**Transitive path:** `<workspace crate>` → `<dep>` → … → `<affected crate>`
**Reachable in QASH:** Yes / No / Conditional (`<condition>`)
**Domain:** A (consensus) / B (pal/address/model) / Tests / Tooling
**Severity:** Critical / High / Medium / Low / Informational
**Exploitability in QASH context:** <explanation>
**Fix available:** Yes (upgrade to `<version>`) / No / Partial
**Decision:** Upgrade / Mitigated / Accept / Blocked-pending-upstream
**Owner sign-off:** @<github-handle>
**Date:** YYYY-MM-DD
**Notes:** <optional additional context>
```

---

## Domain classification

| Domain | Crates |
|--------|--------|
| **A** | `qash-consensus` and its `[dependencies]` (no_std path) |
| **B** | `qash-pal`, `qash-address`, `model`, `src/` and their dependencies |
| **Tests** | `[dev-dependencies]` only; not in production binary |
| **Tooling** | Build scripts, proc-macros, CI tooling; not in deployed artifacts |

---

## Triage sources

Before genesis-lock, run and triage all of the following:

1. **`cargo audit`** — checks against the RustSec advisory database
2. **OSV scan** (GitHub Actions `google/osv-scanner-action`) — broader advisory coverage
3. **`cargo deny check advisories`** — advisory policy enforcement (see `deny.toml`)
4. **`cargo deny check licenses`** — license compatibility gate
5. **`cargo deny check bans`** — banned crate policy
6. **`cargo deny check sources`** — allowed registry/source policy

---

## Current entries

*No open dependency-risk entries as of 2026-05-27. The advisory CI jobs
(`osv-scan`, OSV vulnerability scan, and `supply-chain`) run on every PR. When
any advisory job reports a finding, triage it here within one sprint of
detection.*

---

## Triage decision taxonomy

| Decision | Meaning |
|----------|---------|
| **Upgrade** | Update to a fixed version; verified clean in QASH build |
| **Mitigated** | Vulnerable code path not reachable in QASH (documented below) |
| **Accept** | Low severity; exploitability is nil in QASH context; owner sign-off required |
| **Blocked-pending-upstream** | No fix available; tracking upstream; compensating control documented |

---

## Compensating controls

When a finding is **Mitigated** or **Blocked-pending-upstream**, document the compensating control:

- Code path analysis showing the vulnerable function is unreachable
- Feature flag or cfg exclusion that eliminates the vulnerable dependency
- Domain isolation (Domain A has no std/net/crypto deps; Domain B advisory findings are lower impact on consensus integrity)
- Scheduled re-review date (no more than 90 days for Medium+)

---

## Audit history

| Date | Auditor | Tool | Findings | Triaged | Notes |
|------|---------|------|----------|---------|-------|
| 2026-05-27 | Codex | `cargo audit --deny warnings` | 0 advisories | Yes | Installed `cargo-audit v0.22.1`; fetched RustSec database into `/tmp/qash-cargo-audit`; scanned current `Cargo.lock` with 135 crate dependencies. |
| 2026-05-27 | Codex | `cargo deny check` | 0 blocking findings | Yes | `advisories`, `bans`, `licenses`, and `sources` passed. Existing configuration warnings remain: unmatched `0BSD`/`ISC` allow-list entries and unnecessary `hybrid-array` skip. |
| 2026-05-27 | GitHub Actions | OSV vulnerability scan | 0 blocking findings | Yes | PR #202 OSV vulnerability scan passed; PR-local OSV results remain the source for branch-specific dependency deltas. |

---

## Genesis-lock gate

Before genesis-lock:

- [x] `cargo audit` run on current `Cargo.lock` — zero unacknowledged findings
- [x] OSV scan run — zero unacknowledged findings
- [x] `cargo deny check` passes (or all failures have entries here with owner sign-off)
- [x] All **Critical** and **High** severity findings have `Upgrade` or `Mitigated` status
- [x] All **Medium** findings have a documented decision and owner sign-off
- [x] All **Blocked-pending-upstream** entries have a compensating control and re-review date
